#![no_std]
#![no_main]
#![forbid(unsafe_code)]

mod debug_ui;
mod timer_ui;

use embassy_futures::block_on;
use embedded_graphics::{pixelcolor::Rgb565, prelude::RgbColor};
use embedded_hal::{delay::DelayNs, digital::OutputPin};
use embedded_hal_02::adc::OneShot;
use gc9a01a_driver::{FrameBuffer, GC9A01A, Orientation};
use panic_halt as _;
use ph_qmi8658::{
    AccelConfig, AccelOutputDataRate, AccelRange, Config, I2cConfig, Qmi8658Address, Qmi8658I2c,
};
use shottimer::{
    battery::BatteryStatus,
    diagnostics::{MotionStats, PeakWindow, SAMPLE_COUNT},
    settings::{
        METERS_PER_SECOND_SQUARED_PER_COUNT, RECENT_PEAK_WINDOWS, SAMPLE_DELAY_MS,
        VIBRATION_THRESHOLD,
    },
    shot_timer::ShotTimer,
    ui_mode::{FlipModeSwitch, UiMode},
};
use static_cell::StaticCell;
use waveshare_rp2040_lcd_1_28::{
    Pins, XOSC_CRYSTAL_FREQ, entry,
    hal::{
        self, Sio,
        clocks::{Clock, init_clocks_and_plls},
        fugit::RateExtU32,
        i2c::I2C,
        pac,
        timer::Timer,
        watchdog::Watchdog,
    },
};

use crate::debug_ui::{
    DYNAMIC_REGIONS as DEBUG_REGIONS, DebugStatus, IMU_ERROR_REGION, READ_ERROR_REGION,
    draw_frame as draw_debug_frame, draw_imu_error, draw_read_error, draw_values,
};
use crate::timer_ui::{
    DYNAMIC_REGIONS as TIMER_REGIONS, TimerView, draw_frame as draw_timer_frame,
    draw_view as draw_timer_view,
};

const LCD_SIZE: u32 = 240;
const LCD_BUFFER_BYTES: usize = (LCD_SIZE * LCD_SIZE * 2) as usize;
const BATTERY_SAMPLES: u32 = 8;
static FRAME_STORAGE: StaticCell<[u8; LCD_BUFFER_BYTES]> = StaticCell::new();

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

    // Exact pin/peripheral setup from the upstream Waveshare BSP display demo.
    let lcd_dc = pins.gp8.into_push_pull_output();
    let lcd_cs = pins.gp9.into_push_pull_output();
    let lcd_clk = pins.gp10.into_function::<hal::gpio::FunctionSpi>();
    let lcd_mosi = pins.gp11.into_function::<hal::gpio::FunctionSpi>();
    let lcd_rst = pins
        .gp12
        .into_push_pull_output_in_state(hal::gpio::PinState::High);
    let mut backlight = pins
        .gp25
        .into_push_pull_output_in_state(hal::gpio::PinState::Low);
    let spi = hal::Spi::<_, _, _, 8>::new(pac.SPI1, (lcd_mosi, lcd_clk)).init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        40.MHz(),
        embedded_hal::spi::MODE_0,
    );
    let mut display = GC9A01A::new(spi, lcd_dc, lcd_cs, lcd_rst, false, LCD_SIZE, LCD_SIZE);
    display.init(&mut timer).unwrap();
    display.set_orientation(&Orientation::Portrait).unwrap();

    let frame_storage = FRAME_STORAGE.init_with(|| [0; LCD_BUFFER_BYTES]);
    let mut frame = FrameBuffer::new(frame_storage, LCD_SIZE, LCD_SIZE);
    backlight.set_high().unwrap();
    for color in [Rgb565::RED, Rgb565::GREEN, Rgb565::BLUE] {
        frame.clear(color);
        display.show(frame.get_buffer()).unwrap();
        timer.delay_ms(1_000);
    }
    frame.clear(Rgb565::BLACK);
    draw_timer_frame(&mut frame);
    display.show(frame.get_buffer()).unwrap();

    let mut adc = hal::Adc::new(pac.ADC, &mut pac.RESETS);
    let mut battery_pin = hal::adc::AdcPin::new(pins.gp29).unwrap();

    let sda = pins.gp6.into_function();
    let scl = pins.gp7.into_function();
    let i2c = I2C::new_controller(
        pac.I2C1,
        sda,
        scl,
        400_000u32.Hz(),
        &mut pac.RESETS,
        clocks.system_clock.freq(),
    );
    let accel = AccelConfig::new(AccelRange::G8, AccelOutputDataRate::Hz1000);
    let config = Config::new().with_accel_config(accel).without_gyro();
    let i2c_config = I2cConfig::new(Qmi8658Address::Primary.addr());
    let mut imu: Qmi8658I2c<_> = Qmi8658I2c::with_i2c_config(i2c, None, None, config, i2c_config);
    let addresses = [
        Qmi8658Address::Primary.addr(),
        Qmi8658Address::Secondary.addr(),
    ];
    let imu_address = {
        let mut async_delay = SyncDelay(&mut timer);
        match block_on(imu.init_with_addresses(&mut async_delay, &addresses)) {
            Ok(address) => address,
            Err(_) => {
                draw_imu_error(&mut frame);
                let (x, y, width, height) = IMU_ERROR_REGION;
                display
                    .show_region(frame.get_buffer(), x, y, width, height)
                    .ok();
                loop {
                    timer.delay_ms(1_000);
                }
            }
        }
    };

    let now_ms = milliseconds(&timer);
    let mut shot_timer = ShotTimer::new(now_ms);
    let mut previous_state = shot_timer.state();
    let mut recent_peaks = PeakWindow::<RECENT_PEAK_WINDOWS>::new();
    let mut mode_switch = FlipModeSwitch::new(UiMode::Timer);
    let mut previous_timer_view = None;

    loop {
        let mut samples = [[0.0; 3]; SAMPLE_COUNT];
        let mut read_ok = true;
        for sample in &mut samples {
            match block_on(imu.read_accel_raw()) {
                Ok(value) => {
                    sample[0] = value.data.x as f32 * METERS_PER_SECOND_SQUARED_PER_COUNT;
                    sample[1] = value.data.y as f32 * METERS_PER_SECOND_SQUARED_PER_COUNT;
                    sample[2] = value.data.z as f32 * METERS_PER_SECOND_SQUARED_PER_COUNT;
                }
                Err(_) => read_ok = false,
            }
            timer.delay_ms(SAMPLE_DELAY_MS);
        }

        if !read_ok {
            draw_read_error(&mut frame, imu_address);
            let (x, y, width, height) = READ_ERROR_REGION;
            display
                .show_region(frame.get_buffer(), x, y, width, height)
                .ok();
            continue;
        }

        let mut battery_total = 0u32;
        for _ in 0..BATTERY_SAMPLES {
            let counts: u16 = adc.read(&mut battery_pin).unwrap();
            battery_total += u32::from(counts);
        }
        let battery = BatteryStatus::from_adc_counts((battery_total / BATTERY_SAMPLES) as u16);

        let stats = MotionStats::from_samples(&samples);
        let recent_peak = recent_peaks.push(stats.peak_deviation);
        let now_ms = milliseconds(&timer);
        let mut full_refresh = false;
        let mode_change = mode_switch.update(stats.mean);
        let screen_direction = mode_switch.screen_direction(stats.mean);
        if let Some(new_mode) = mode_change {
            previous_timer_view = None;
            frame.clear(Rgb565::BLACK);
            match new_mode {
                UiMode::Timer => draw_timer_frame(&mut frame),
                UiMode::Debug => draw_debug_frame(&mut frame),
            }
            full_refresh = true;
        }

        let state = match mode_change {
            Some(UiMode::Debug) => shot_timer.reset(now_ms),
            Some(UiMode::Timer) => shot_timer.reset(now_ms),
            None => shot_timer.update(now_ms, stats.is_vibrating(VIBRATION_THRESHOLD)),
        };
        if state != previous_state {
            if state.is_sleeping() {
                backlight.set_low().ok();
            } else {
                backlight.set_high().ok();
            }
            previous_state = state;
        }
        if !state.is_sleeping() {
            match mode_switch.mode() {
                UiMode::Debug => {
                    draw_values(
                        &mut frame,
                        imu_address,
                        &stats,
                        recent_peak,
                        DebugStatus {
                            battery,
                            shot_state: state,
                            now_ms,
                            screen_direction,
                        },
                    );
                    if full_refresh {
                        display.show(frame.get_buffer()).ok();
                    } else {
                        show_regions(&mut display, &frame, &DEBUG_REGIONS);
                    }
                }
                UiMode::Timer => {
                    let view = TimerView::new(state, now_ms);
                    if full_refresh || previous_timer_view != Some(view) {
                        draw_timer_view(&mut frame, view);
                        if full_refresh {
                            display.show(frame.get_buffer()).ok();
                        } else {
                            show_regions(&mut display, &frame, &TIMER_REGIONS);
                        }
                        previous_timer_view = Some(view);
                    }
                }
            }
        }
    }
}

fn show_regions<SPI, DC, CS, RST>(
    display: &mut GC9A01A<SPI, DC, CS, RST>,
    frame: &FrameBuffer<'_>,
    regions: &[(u16, u16, u32, u32)],
) where
    SPI: embedded_hal::spi::SpiBus<u8>,
    DC: OutputPin,
    CS: OutputPin,
    RST: OutputPin,
{
    for &(x, y, width, height) in regions {
        display
            .show_region(frame.get_buffer(), x, y, width, height)
            .ok();
    }
}

fn milliseconds(timer: &Timer) -> u64 {
    timer.get_counter().ticks() / 1_000
}

struct SyncDelay<'a, D>(&'a mut D);

impl<D: DelayNs> embedded_hal_async::delay::DelayNs for SyncDelay<'_, D> {
    async fn delay_ns(&mut self, ns: u32) {
        self.0.delay_ns(ns);
    }
}
