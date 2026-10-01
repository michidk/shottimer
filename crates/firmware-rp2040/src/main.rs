#![no_std]
#![no_main]
#![forbid(unsafe_code)]
mod hardware;
use core::cell::RefCell;
use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
    pwm::SetDutyCycle,
    spi::SpiBus,
};
use embedded_hal_02::adc::OneShot;
use embedded_hal_async::i2c::I2c;
use panic_halt as _;
use shottimer_app::{
    BatteryReading, HardwareError, Platform, Region,
    drivers::{Display, Imu},
};
use shottimer_core::settings::{DISPLAY_BRIGHTNESS_PERCENT, SLEEP_CHECK_INTERVAL_MS};
use usb_device::{
    class_prelude::UsbBusAllocator,
    device::{StringDescriptors, UsbDevice, UsbDeviceBuilder, UsbDeviceState, UsbVidPid},
};
use usbd_serial::SerialPort;
use waveshare_rp2040_lcd_1_28::{
    Pins, XOSC_CRYSTAL_FREQ, entry,
    hal::{
        self, Sio,
        clocks::{Clock, init_clocks_and_plls},
        fugit::{ExtU32, RateExtU32},
        i2c::I2C,
        pac,
        timer::{Alarm, Timer},
        watchdog::Watchdog,
    },
};
#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let clocks = init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();
    let sio = Sio::new(pac.SIO);
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    let mut timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let mut core = cortex_m::Peripherals::take().unwrap();
    // Masked NVIC interrupts still generate events with SEVONPEND. This lets
    // WFE stop the CPU without unsafe interrupt handlers or losing a wake edge.
    core.SCB.set_sevonpend();
    let sleep_alarm = timer.alarm_0().unwrap();

    let (
        lcd_dc,
        lcd_cs,
        lcd_clk,
        lcd_mosi,
        lcd_rst,
        backlight,
        battery_gpio,
        sda,
        scl,
        imu_irq,
        lcd_spi,
        imu_i2c,
    ) = hardware::select_hardware!(pins, pac);

    let lcd_dc = lcd_dc.into_push_pull_output();
    let lcd_cs = lcd_cs.into_push_pull_output();
    let lcd_clk = lcd_clk.into_function::<hal::gpio::FunctionSpi>();
    let lcd_mosi = lcd_mosi.into_function::<hal::gpio::FunctionSpi>();
    let lcd_rst = lcd_rst.into_push_pull_output_in_state(hal::gpio::PinState::High);
    let mut pwm_slices = hal::pwm::Slices::new(pac.PWM, &mut pac.RESETS);
    let backlight_pwm = hardware::select_backlight_pwm!(pwm_slices);
    backlight_pwm.set_duty_cycle(0).unwrap();
    backlight_pwm.output_to(backlight);
    let spi = hal::Spi::<_, _, _, 8>::new(lcd_spi, (lcd_mosi, lcd_clk)).init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        40.MHz(),
        embedded_hal::spi::MODE_0,
    );
    let spi = RefCell::new(spi);
    let lcd_dc = RefCell::new(lcd_dc);
    let lcd_cs = RefCell::new(lcd_cs);
    let display = Display::new(&spi, &lcd_dc, &lcd_cs, lcd_rst, &mut timer).unwrap();
    let mut adc = hal::Adc::new(pac.ADC, &mut pac.RESETS);
    let battery_gpio = battery_gpio.into_pull_type::<hal::gpio::PullNone>();
    let mut battery_pin = hal::adc::AdcPin::new(battery_gpio).unwrap();

    let sda = sda.into_function();
    let imu_irq = imu_irq.into_floating_input();
    let scl = scl.into_function();
    let i2c = I2C::new_controller(
        imu_i2c,
        sda,
        scl,
        400_000u32.Hz(),
        &mut pac.RESETS,
        clocks.system_clock.freq(),
    );
    let imu = Imu::new(i2c);
    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));
    let serial = SerialPort::new(&usb_bus);
    let usb = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .strings(&[StringDescriptors::default()
            .manufacturer("Shottimer")
            .product("Shottimer Debug Serial")
            .serial_number("SHOT1")])
        .unwrap()
        .device_class(2)
        .build();
    let battery = move || {
        let mut total = 0u32;
        for _ in 0..32 {
            let count: u16 = adc
                .read(&mut battery_pin)
                .map_err(|_| HardwareError::Battery)?;
            total += u32::from(count);
        }
        let raw = (total / 32) as u16;
        Ok(BatteryReading {
            raw_counts: raw,
            voltage: shottimer_core::battery::BatteryStatus::from_adc_counts(raw).voltage,
        })
    };
    let mut imu_irq = imu_irq;
    let mut sleep_alarm = sleep_alarm;
    // The closure owns the RP-specific interrupt and wake-alarm plumbing.
    let wake = move |operation: WakeOperation| match operation {
        WakeOperation::Pending => imu_irq.is_high().unwrap_or(false),
        WakeOperation::Enable => {
            imu_irq.clear_interrupt(hal::gpio::Interrupt::EdgeHigh);
            pac::NVIC::unpend(pac::Interrupt::IO_IRQ_BANK0);
            imu_irq.set_interrupt_enabled(hal::gpio::Interrupt::EdgeHigh, true);
            false
        }
        WakeOperation::Wait => {
            sleep_alarm.clear_interrupt();
            pac::NVIC::unpend(pac::Interrupt::TIMER_IRQ_0);
            imu_irq.clear_interrupt(hal::gpio::Interrupt::EdgeHigh);
            pac::NVIC::unpend(pac::Interrupt::IO_IRQ_BANK0);
            sleep_alarm
                .schedule(SLEEP_CHECK_INTERVAL_MS.millis())
                .unwrap();
            sleep_alarm.enable_interrupt();
            cortex_m::asm::wfe();
            false
        }
        WakeOperation::Disable => {
            imu_irq.set_interrupt_enabled(hal::gpio::Interrupt::EdgeHigh, false);
            sleep_alarm.disable_interrupt();
            sleep_alarm.clear_interrupt();
            pac::NVIC::unpend(pac::Interrupt::TIMER_IRQ_0);
            pac::NVIC::unpend(pac::Interrupt::IO_IRQ_BANK0);
            false
        }
    };
    shottimer_app::run(Board {
        display,
        imu,
        timer,
        pwm: backlight_pwm,
        battery,
        wake,
        usb,
        serial,
    });
}

enum WakeOperation {
    Pending,
    Enable,
    Wait,
    Disable,
}

struct Board<
    'a,
    S: SpiBus<u8>,
    D: OutputPin,
    C: OutputPin,
    R: OutputPin,
    I: I2c,
    P: SetDutyCycle,
    B,
    W,
> {
    display: Display<'a, S, D, C, R>,
    imu: Imu<I>,
    timer: Timer,
    pwm: P,
    battery: B,
    wake: W,
    usb: UsbDevice<'a, hal::usb::UsbBus>,
    serial: SerialPort<'a, hal::usb::UsbBus>,
}
impl<S: SpiBus<u8>, D: OutputPin, C: OutputPin, R: OutputPin, I: I2c, P: SetDutyCycle, B, W>
    Platform for Board<'_, S, D, C, R, I, P, B, W>
where
    B: FnMut() -> Result<BatteryReading, HardwareError>,
    W: FnMut(WakeOperation) -> bool,
{
    fn now_ms(&self) -> u64 {
        self.timer.get_counter().ticks() / 1000
    }
    fn delay_ms(&mut self, ms: u32) {
        self.timer.delay_ms(ms);
    }
    fn initialize_imu(&mut self) -> Result<u8, HardwareError> {
        self.imu.initialize(&mut self.timer)
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
        self.pwm
            .set_duty_cycle(if on {
                DISPLAY_BRIGHTNESS_PERCENT as u16 * 10
            } else {
                0
            })
            .ok();
    }
    fn poll(&mut self) {
        self.usb.poll(&mut [&mut self.serial]);
    }
    fn externally_powered(&self) -> bool {
        self.usb.state() == UsbDeviceState::Configured
    }
    fn logging_connected(&self) -> bool {
        self.serial.dtr()
    }
    fn log(&mut self, b: &[u8]) {
        if self.serial.dtr() {
            self.serial.write(b).ok();
        }
    }
    fn enter_sleep(&mut self) -> Result<(), HardwareError> {
        self.imu.enter_sleep(&mut self.timer)?;
        self.display.sleep(&mut self.timer)?;
        (self.wake)(WakeOperation::Enable);
        Ok(())
    }
    fn motion_pending(&mut self) -> Result<bool, HardwareError> {
        if (self.wake)(WakeOperation::Pending) {
            Ok(true)
        } else {
            self.imu.motion_pending()
        }
    }
    fn wait_for_wake(&mut self) {
        (self.wake)(WakeOperation::Wait);
    }
    fn exit_sleep(&mut self) -> Result<(), HardwareError> {
        (self.wake)(WakeOperation::Disable);
        self.imu.exit_sleep(&mut self.timer)?;
        self.display.wake(&mut self.timer)
    }
}
