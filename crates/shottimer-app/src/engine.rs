//! Shared firmware loop: board access is restricted to the Platform interface.
use crate::{
    Platform,
    debug_ui::{
        DYNAMIC_REGIONS as DEBUG_REGIONS, DebugStatus, IMU_ERROR_REGION, READ_ERROR_REGION,
        draw_frame as draw_debug_frame, draw_imu_error, draw_read_error, draw_values,
    },
    timer_ui::{
        DYNAMIC_REGIONS as TIMER_REGIONS, TimerView, draw_frame as draw_timer_frame,
        draw_view as draw_timer_view,
    },
};
use core::fmt::Write;
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_10X20},
    pixelcolor::Rgb565,
    prelude::*,
    text::{Alignment, Text},
};
use gc9a01a_driver::FrameBuffer;
use heapless::String;
use shottimer_core::{
    battery::BatteryMonitor,
    diagnostics::{MotionStats, PeakWindow, SAMPLE_COUNT},
    settings::*,
    shot_timer::ShotTimer,
    ui_mode::{FlipModeSwitch, OrientationCalibration, ScreenAxis, UiMode},
};
use static_cell::StaticCell;
const LCD_SIZE: u32 = 240;
const LCD_BUFFER_BYTES: usize = (LCD_SIZE * LCD_SIZE * 2) as usize;
const CALIBRATION_SAMPLES: u32 = CALIBRATION_DURATION_MS / SAMPLE_DELAY_MS;
const CALIBRATION_DISPLAY_INTERVAL_SAMPLES: u32 = 10;
static FRAME_STORAGE: StaticCell<[u8; LCD_BUFFER_BYTES]> = StaticCell::new();
pub fn run<P: Platform>(mut board: P) -> ! {
    let storage = FRAME_STORAGE.init_with(|| [0; LCD_BUFFER_BYTES]);
    let mut frame = FrameBuffer::new(storage, LCD_SIZE, LCD_SIZE);
    board.backlight(true);
    let imu_address = match board.initialize_imu() {
        Ok(address) => address,
        Err(_) => {
            draw_imu_error(&mut frame);
            board
                .show(frame.get_buffer(), Some(&[IMU_ERROR_REGION]))
                .ok();
            loop {
                board.poll();
                board.delay_ms(1000);
            }
        }
    };
    if COLOR_TEST_ENABLED {
        for color in [Rgb565::RED, Rgb565::GREEN, Rgb565::BLUE] {
            frame.clear(color);
            board.show(frame.get_buffer(), None).unwrap();
            board.delay_ms(1_000);
        }
    }

    let mut imu_read_errors = 0u32;
    let mut calibration = OrientationCalibration::new();
    frame.clear(Rgb565::BLACK);
    draw_calibrating(&mut frame, [0.0; 3], calibration.sample_count());
    board.show(frame.get_buffer(), None).unwrap();
    for attempt in 1..=CALIBRATION_SAMPLES {
        if let Ok(value) = board.read_accel_raw() {
            calibration.add([value[0] as f32, value[1] as f32, value[2] as f32]);
        } else {
            imu_read_errors = imu_read_errors.saturating_add(1);
        }
        board.delay_ms(SAMPLE_DELAY_MS);

        if attempt % CALIBRATION_DISPLAY_INTERVAL_SAMPLES == 0 {
            frame.clear(Rgb565::BLACK);
            draw_calibrating(
                &mut frame,
                calibration.mean().unwrap_or([0.0; 3]),
                calibration.sample_count(),
            );
            board.show(frame.get_buffer(), None).ok();
        }
    }
    let screen_up_mean = calibration.mean().unwrap_or([0.0, 0.0, 1.0]);

    frame.clear(Rgb565::BLACK);
    let initial_mode = if DEBUG_MODE_ENABLED && START_IN_DEBUG_MODE {
        UiMode::Debug
    } else {
        UiMode::Timer
    };
    match initial_mode {
        UiMode::Timer => draw_timer_frame(&mut frame),
        UiMode::Debug => draw_debug_frame(&mut frame),
    }
    board.show(frame.get_buffer(), None).unwrap();

    let now_ms = board.now_ms();
    let mut shot_timer = ShotTimer::new(now_ms);
    let mut battery_monitor = BatteryMonitor::new();
    let mut previous_state = shot_timer.state();
    let mut recent_peaks = PeakWindow::<RECENT_PEAK_WINDOWS>::new();
    let mut mode_switch = FlipModeSwitch::calibrated(initial_mode, screen_up_mean);
    let mut previous_timer_view = None;
    let mut last_serial_log_ms = 0;
    let mut low_power = false;

    loop {
        if low_power {
            // No LCD writes or normal sample windows asleep.
            board.poll();
            // Battery measurements never prevent sleep or wake the board.
            let powered = board.externally_powered();
            let motion = board.motion_pending().unwrap_or_else(|_| {
                imu_read_errors = imu_read_errors.saturating_add(1);
                true
            });
            if !motion && !board.logging_connected() && !powered {
                board.wait_for_wake();
                continue;
            }
            if board.exit_sleep().is_err() {
                imu_read_errors = imu_read_errors.saturating_add(1);
                board.delay_ms(100);
                continue;
            }
            shot_timer.reset(board.now_ms());
            previous_state = shot_timer.state();
            previous_timer_view = None;
            recent_peaks = PeakWindow::new();
            board.show(frame.get_buffer(), None).ok();
            board.backlight(true);
            low_power = false;
        }
        let mut samples = [[0.0; 3]; SAMPLE_COUNT];
        let mut read_ok = true;
        for sample in &mut samples {
            board.poll();
            match board.read_accel_raw() {
                Ok(value) => {
                    sample[0] = value[0] as f32 * METERS_PER_SECOND_SQUARED_PER_COUNT;
                    sample[1] = value[1] as f32 * METERS_PER_SECOND_SQUARED_PER_COUNT;
                    sample[2] = value[2] as f32 * METERS_PER_SECOND_SQUARED_PER_COUNT;
                }
                Err(_) => {
                    read_ok = false;
                    imu_read_errors = imu_read_errors.saturating_add(1);
                }
            }
            board.delay_ms(SAMPLE_DELAY_MS);
        }

        if !read_ok {
            draw_read_error(&mut frame, imu_read_errors);
            board
                .show(frame.get_buffer(), Some(&[READ_ERROR_REGION]))
                .ok();
            continue;
        }

        let reading = read_battery(&mut board);
        let stats = MotionStats::from_samples(&samples);
        let recent_peak = recent_peaks.push(stats.peak_deviation);
        let now_ms = board.now_ms();
        let battery = battery_monitor.update_voltage(reading.raw_counts, reading.voltage, now_ms);
        let powered = board.externally_powered();
        let charge_percent = if (powered || battery.charging_indicated()) && battery.connected {
            Some(battery.charge_percent)
        } else {
            None
        };
        let mut full_refresh = false;
        let mode_change = mode_switch.update(now_ms, stats.mean);
        let screen_direction = mode_switch.screen_direction(stats.mean);
        let screen_vertical = mode_switch.screen_vertical(stats.mean);
        let screen_axis = mode_switch.screen_axis();
        if USB_LOGGING_ENABLED && now_ms.saturating_sub(last_serial_log_ms) >= USB_LOG_INTERVAL_MS {
            last_serial_log_ms = now_ms;
            write_orientation_log(
                &mut board,
                &stats,
                screen_axis,
                mode_switch.screen_raw_vertical(stats.mean),
                screen_vertical,
                screen_direction,
            );
        }
        if let Some(new_mode) = mode_change {
            previous_timer_view = None;
            frame.clear(Rgb565::BLACK);
            match new_mode {
                UiMode::Timer => draw_timer_frame(&mut frame),
                UiMode::Debug => draw_debug_frame(&mut frame),
            }
            full_refresh = true;
        }

        let state = if mode_change.is_some() {
            shot_timer.reset(now_ms)
        } else {
            shot_timer.update_with_sleep_policy(
                now_ms,
                stats.is_vibrating(VIBRATION_SENSITIVITY_THRESHOLD),
                SLEEP_ENABLED && !powered,
            )
        };
        if state != previous_state
            || (state.is_sleeping() && !board.logging_connected() && !low_power)
        {
            if state.is_sleeping() {
                board.backlight(false);
                // Keep normal acquisition while a serial debugger is attached.
                if !board.logging_connected() {
                    if board.enter_sleep().is_ok() {
                        low_power = true;
                    } else {
                        imu_read_errors = imu_read_errors.saturating_add(1);
                        board.exit_sleep().ok();
                        shot_timer.reset(board.now_ms());
                    }
                }
            } else {
                board.backlight(true);
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
                            screen_vertical,
                            screen_axis,
                            imu_read_errors,
                        },
                    );
                    if full_refresh {
                        board.show(frame.get_buffer(), None).ok();
                    } else {
                        board.show(frame.get_buffer(), Some(&DEBUG_REGIONS)).ok();
                    }
                }
                UiMode::Timer => {
                    let view =
                        TimerView::new(state, now_ms, shot_timer.previous_shots(), charge_percent);
                    if full_refresh || previous_timer_view != Some(view) {
                        draw_timer_view(&mut frame, view);
                        if full_refresh {
                            board.show(frame.get_buffer(), None).ok();
                        } else {
                            board.show(frame.get_buffer(), Some(&TIMER_REGIONS)).ok();
                        }
                        previous_timer_view = Some(view);
                    }
                }
            }
        }
    }
}

fn read_battery<P: Platform>(board: &mut P) -> crate::BatteryReading {
    let empty = crate::BatteryReading {
        raw_counts: 0,
        voltage: 0.0,
    };
    if USE_BATTERY {
        board.read_battery().unwrap_or(empty)
    } else {
        empty
    }
}

fn draw_calibrating<D>(display: &mut D, average_raw: [f32; 3], samples: u32)
where
    D: DrawTarget<Color = Rgb565>,
{
    let style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
    for (text, y) in [
        ("CALIBRATING IMU", 70),
        ("HOLD SCREEN FACE UP", 95),
        ("KEEP STILL", 120),
    ] {
        Text::with_alignment(text, Point::new(120, y), style, Alignment::Center)
            .draw(display)
            .ok();
    }

    let acceleration = average_raw.map(|value| value * METERS_PER_SECOND_SQUARED_PER_COUNT);
    let mut line = String::<32>::new();
    write!(line, "X {:+.2}  Y {:+.2}", acceleration[0], acceleration[1]).ok();
    Text::with_alignment(&line, Point::new(120, 150), style, Alignment::Center)
        .draw(display)
        .ok();

    line.clear();
    write!(line, "Z {:+.2} m/s2", acceleration[2]).ok();
    Text::with_alignment(&line, Point::new(120, 175), style, Alignment::Center)
        .draw(display)
        .ok();

    line.clear();
    write!(line, "SAMPLES {samples}/{CALIBRATION_SAMPLES}").ok();
    Text::with_alignment(&line, Point::new(120, 200), style, Alignment::Center)
        .draw(display)
        .ok();
}

fn write_orientation_log<P: Platform>(
    board: &mut P,
    stats: &MotionStats,
    axis: ScreenAxis,
    raw_vertical: f32,
    filtered_vertical: f32,
    direction: shottimer_core::ui_mode::ScreenDirection,
) {
    let mut line = String::<128>::new();
    writeln!(
        line,
        "imu x={:+.2} y={:+.2} z={:+.2}\r",
        stats.mean[0], stats.mean[1], stats.mean[2]
    )
    .ok();
    writeln!(
        line,
        "orient axis={} raw={raw_vertical:+.2} filt={filtered_vertical:+.2} {direction:?}\r",
        axis.label()
    )
    .ok();
    board.log(line.as_bytes());
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use crate::{BatteryReading, HardwareError, Region};
    use std::{
        cell::RefCell,
        panic::{AssertUnwindSafe, catch_unwind},
        rc::Rc,
        vec::Vec,
    };

    #[derive(Default)]
    struct Trace {
        colors: Vec<u16>,
        reads: usize,
        battery_reads: usize,
        logs: usize,
        sleeps: usize,
        wakes: usize,
        waits: usize,
    }
    struct FakeBoard {
        now: u64,
        trace: Rc<RefCell<Trace>>,
    }
    impl Platform for FakeBoard {
        fn now_ms(&self) -> u64 {
            self.now
        }
        fn delay_ms(&mut self, ms: u32) {
            self.now += u64::from(ms);
        }
        fn initialize_imu(&mut self) -> Result<u8, HardwareError> {
            Ok(0x6b)
        }
        fn read_accel_raw(&mut self) -> Result<[i16; 3], HardwareError> {
            let mut trace = self.trace.borrow_mut();
            trace.reads += 1;
            let z = if (85_000..87_000).contains(&self.now) {
                -4096
            } else {
                4096
            };
            let vibration = if (7_000..12_000).contains(&self.now) && trace.reads.is_multiple_of(2)
            {
                1500
            } else {
                0
            };
            Ok([vibration, 0, z])
        }
        fn read_battery(&mut self) -> Result<BatteryReading, HardwareError> {
            let mut trace = self.trace.borrow_mut();
            assert_eq!(
                trace.sleeps, trace.wakes,
                "battery must not be sampled asleep"
            );
            trace.battery_reads += 1;
            Ok(BatteryReading {
                raw_counts: 2400,
                // Sustained rising voltage must not block idle sleep or wake it.
                voltage: 3.85 + self.now as f32 * 0.000_000_1,
            })
        }
        fn show(&mut self, buf: &[u8], regions: Option<&[Region]>) -> Result<(), HardwareError> {
            let mut trace = self.trace.borrow_mut();
            if COLOR_TEST_ENABLED && regions.is_none() && trace.colors.len() < 3 {
                let pixel = u16::from_be_bytes([buf[0], buf[1]]);
                assert!(
                    buf.as_chunks::<2>()
                        .0
                        .iter()
                        .all(|p| u16::from_be_bytes(*p) == pixel)
                );
                trace.colors.push(pixel);
            }
            Ok(())
        }
        fn backlight(&mut self, _: bool) {}
        fn poll(&mut self) {
            if self.now >= 90_000 {
                std::panic::panic_any("simulation complete");
            }
        }
        fn externally_powered(&self) -> bool {
            false
        }
        fn logging_connected(&self) -> bool {
            false
        }
        fn log(&mut self, bytes: &[u8]) {
            assert!(bytes.starts_with(b"imu x="));
            self.trace.borrow_mut().logs += 1;
        }
        fn enter_sleep(&mut self) -> Result<(), HardwareError> {
            self.trace.borrow_mut().sleeps += 1;
            Ok(())
        }
        fn motion_pending(&mut self) -> Result<bool, HardwareError> {
            Ok(self.now >= 80_000)
        }
        fn wait_for_wake(&mut self) {
            self.trace.borrow_mut().waits += 1;
            self.now += 500;
        }
        fn exit_sleep(&mut self) -> Result<(), HardwareError> {
            self.trace.borrow_mut().wakes += 1;
            Ok(())
        }
    }

    #[test]
    fn shared_loop_runs_boot_calibration_sampling_logging_and_sleep_recovery() {
        let trace = Rc::new(RefCell::new(Trace::default()));
        let result = catch_unwind(AssertUnwindSafe(|| {
            run(FakeBoard {
                now: 0,
                trace: trace.clone(),
            })
        }));
        let reason = result.unwrap_err();
        assert_eq!(reason.downcast_ref::<&str>(), Some(&"simulation complete"));
        let trace = trace.borrow();
        if COLOR_TEST_ENABLED {
            assert_eq!(trace.colors, [0xf800, 0x07e0, 0x001f]);
        } else {
            assert!(trace.colors.is_empty());
        }
        assert!(trace.reads >= 300);
        assert_eq!(trace.battery_reads > 0, USE_BATTERY);
        assert!(trace.logs > 100);
        assert_eq!(trace.sleeps, usize::from(SLEEP_ENABLED));
        assert_eq!(trace.wakes, usize::from(SLEEP_ENABLED));
        assert_eq!(trace.waits > 0, SLEEP_ENABLED);
    }
}
