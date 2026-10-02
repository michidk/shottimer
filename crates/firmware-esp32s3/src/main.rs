#![no_std]
#![no_main]
#![forbid(unsafe_code)]
mod hardware;
use core::cell::RefCell;
use embassy_embedded_hal::adapter::BlockingAsync;
use embedded_hal::delay::DelayNs;
use esp_hal::{
    Blocking,
    analog::adc::{Adc, AdcCalBasic, AdcCalCurve, AdcCalScheme, AdcConfig, Attenuation},
    delay::Delay,
    gpio::{DriveMode, Event, Input, InputConfig, Level, Output, OutputConfig},
    i2c::master::{Config as I2cConfig, I2c},
    ledc::{
        LSGlobalClkSource, Ledc, LowSpeed,
        channel::{self, ChannelIFace},
        timer::{self, TimerIFace},
    },
    rtc_cntl::sleep::{LowPower, RtcSleepConfig},
    spi::master::{Config as SpiConfig, Spi},
    time::{Duration, Instant, Rate},
    uart::{Config as UartConfig, UartTx},
};
use panic_halt as _;
use shottimer_app::{
    BatteryReading, HardwareError, Platform, Region, WakeStatus,
    drivers::{self, Display, Imu},
};
use shottimer_core::settings::{DISPLAY_BRIGHTNESS_PERCENT, SLEEP_CHECK_INTERVAL_MS};
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal::main]
fn main() -> ! {
    let p = esp_hal::init(esp_hal::Config::default());
    let (
        dc,
        cs,
        clk,
        mosi,
        rst,
        bl,
        bat,
        sda,
        scl,
        irq,
        touch_rst,
        tx,
        spi_port,
        i2c_port,
        uart_port,
    ) = hardware::select_hardware!(p);
    let delay = Delay::new();
    let spi = RefCell::new(
        Spi::new(
            spi_port,
            SpiConfig::default().with_frequency(Rate::from_mhz(40)),
        )
        .unwrap()
        .with_sck(clk)
        .with_mosi(mosi),
    );
    let dc = RefCell::new(Output::new(dc, Level::Low, OutputConfig::default()));
    let cs = RefCell::new(Output::new(cs, Level::High, OutputConfig::default()));
    let reset = Output::new(rst, Level::High, OutputConfig::default());
    // The shared runtime initializes the panel and retries on failure.
    let display = Display::new(&spi, &dc, &cs, reset);
    let _touch_reset = Output::new(touch_rst, Level::High, OutputConfig::default());
    let i2c = I2c::new(
        i2c_port,
        I2cConfig::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(sda)
    .with_scl(scl);
    let imu = Imu::new(BlockingAsync::new(i2c));
    let irq = Input::new(irq, InputConfig::default());
    let mut ledc = Ledc::new(p.LEDC);
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);
    let mut pwm_timer = ledc.timer::<LowSpeed>(timer::Number::Timer0);
    pwm_timer
        .configure(timer::config::Config {
            duty: timer::config::Duty::Duty10Bit,
            clock_source: timer::LSClockSource::APBClk,
            frequency: Rate::from_khz(20),
        })
        .unwrap();
    let mut backlight = ledc.channel::<LowSpeed>(channel::Number::Channel0, bl);
    backlight
        .configure(channel::config::Config {
            timer: &pwm_timer,
            duty_pct: 0,
            drive_mode: DriveMode::PushPull,
        })
        .unwrap();
    let mut adc_config = AdcConfig::new();
    let attenuation = Attenuation::_6dB;
    let mut battery_pin = adc_config.enable_pin_with_cal::<_, AdcCalBasic<_>>(bat, attenuation);
    let calibration = AdcCalCurve::<esp_hal::peripherals::ADC1>::new_cal(attenuation);
    let mut adc = Adc::new(p.ADC1, adc_config);
    let battery = move || {
        let mut raw = 0u32;
        let mut millivolts = 0u32;
        for _ in 0..32 {
            let count = adc.read_blocking(&mut battery_pin);
            raw += u32::from(count);
            millivolts += u32::from(calibration.adc_val(count));
        }
        Ok(BatteryReading {
            raw_counts: (raw / 32) as u16,
            voltage: millivolts as f32 / 32.0 / 1000.0 / hardware::BATTERY_DIVIDER_RATIO,
        })
    };
    let serial = UartTx::new(uart_port, UartConfig::default().with_baudrate(115_200))
        .unwrap()
        .with_tx(tx);
    shottimer_app::run(Board {
        display,
        imu,
        delay,
        backlight,
        battery,
        irq,
        low_power: LowPower::new(p.LPWR),
        serial,
    });
}
struct Board<'a, B> {
    display: Display<'a, Spi<'static, Blocking>, Output<'static>, Output<'static>, Output<'static>>,
    imu: Imu<BlockingAsync<I2c<'static, Blocking>>>,
    delay: Delay,
    backlight: channel::Channel<'a, LowSpeed>,
    battery: B,
    irq: Input<'static>,
    low_power: LowPower<'static>,
    serial: UartTx<'static, Blocking>,
}
impl<B: FnMut() -> Result<BatteryReading, HardwareError>> Platform for Board<'_, B> {
    fn now_ms(&self) -> u64 {
        Instant::now().duration_since_epoch().as_millis()
    }
    fn delay_ms(&mut self, ms: u32) {
        self.delay.delay_ms(ms);
    }
    fn initialize_display(&mut self) -> Result<(), HardwareError> {
        self.display.initialize(&mut self.delay)
    }
    fn initialize_imu(&mut self) -> Result<u8, HardwareError> {
        self.imu.initialize(&mut self.delay)
    }
    fn read_accel_raw(&mut self) -> Result<[i16; 3], HardwareError> {
        self.imu.read_raw()
    }
    fn read_battery(&mut self) -> Result<BatteryReading, HardwareError> {
        (self.battery)()
    }
    fn show(&mut self, b: &[u8], r: Option<&[Region]>) -> Result<(), HardwareError> {
        self.display.show(b, r)
    }
    fn backlight(&mut self, on: bool) {
        self.backlight
            .set_duty(if on { DISPLAY_BRIGHTNESS_PERCENT } else { 0 })
            .ok();
    }
    fn poll(&mut self) {}
    // CH343 exposes no USB enumeration or charger-status signal to the MCU.
    fn externally_powered(&self) -> bool {
        false
    }
    fn logging_connected(&self) -> bool {
        false
    }
    fn log(&mut self, bytes: &[u8]) {
        let mut remaining = bytes;
        while !remaining.is_empty() {
            match self.serial.write(remaining) {
                Ok(0) | Err(_) => break,
                Ok(n) => remaining = &remaining[n..],
            }
        }
    }
    fn enter_sleep(&mut self) -> Result<(), HardwareError> {
        self.imu.enter_sleep(&mut self.delay)?;
        self.display.sleep(&mut self.delay)?;
        self.irq.listen(Event::HighLevel);
        Ok(())
    }
    fn wake_status(&mut self) -> Result<WakeStatus, HardwareError> {
        let interrupt_high = self.irq.is_high();
        let motion_detected = self.imu.motion_pending()?;
        Ok(WakeStatus {
            interrupt_high,
            motion_detected,
        })
    }
    fn wait_for_wake(&mut self) {
        self.low_power.set_wakeup_deadline(
            Instant::now() + Duration::from_millis(SLEEP_CHECK_INTERVAL_MS as u64),
        );
        self.low_power.sleep_light(RtcSleepConfig::default());
        self.low_power.clear_wakeup_deadline();
    }
    fn exit_sleep(&mut self) -> Result<Option<HardwareError>, HardwareError> {
        self.irq.unlisten();
        self.irq.clear_interrupt();
        drivers::restore_after_sleep(&mut self.imu, &mut self.display, &mut self.delay)
    }
}
