//! Shared firmware loop: board access is restricted to the Platform interface.
use crate::{
    Platform, Region,
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
    battery::{BatteryMonitor, BatteryStatus},
    diagnostics::{MotionStats, PeakWindow, SAMPLE_COUNT},
    settings::*,
    shot_timer::{ShotState, ShotTimer},
    ui_mode::{CalibrationError, FlipModeSwitch, OrientationCalibration, ScreenAxis, UiMode},
};
use static_cell::StaticCell;
const LCD_SIZE: u32 = 240;
const LCD_BUFFER_BYTES: usize = (LCD_SIZE * LCD_SIZE * 2) as usize;
const CALIBRATION_SAMPLES: u32 = CALIBRATION_DURATION_MS / SAMPLE_DELAY_MS;
const CALIBRATION_DISPLAY_INTERVAL_SAMPLES: u32 = 10;
/// Minimum successful IMU reads for a calibration window to be trusted.
const CALIBRATION_MIN_SAMPLES: u32 = CALIBRATION_SAMPLES / 2;
/// How long a rejected calibration window's reason stays on screen.
const CALIBRATION_RETRY_MS: u32 = 2_000;
/// Consecutive failed LCD transfers before the controller is reinitialized.
const DISPLAY_REINIT_FAILURES: u32 = 3;
/// Wait between boot-time LCD initialization attempts.
const DISPLAY_INIT_RETRY_MS: u32 = 1_000;
static FRAME_STORAGE: StaticCell<[u8; LCD_BUFFER_BYTES]> = StaticCell::new();

pub fn run<P: Platform>(board: P) -> ! {
    let storage = FRAME_STORAGE.init_with(|| [0; LCD_BUFFER_BYTES]);
    let mut app = App::boot(board, FrameBuffer::new(storage, LCD_SIZE, LCD_SIZE));
    loop {
        app.cycle();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Power {
    Active,
    /// Low power, waiting for confirmed motion or a USB wake.
    Sleeping,
    /// A wake failed and the IMU or LCD may still be asleep. Wake sources are
    /// already disabled, so restoration is retried without waiting for motion.
    Restoring,
}

/// USB power and serial-terminal observations for one cycle.
#[derive(Clone, Copy)]
struct Link {
    powered: bool,
    serial: bool,
}

impl Link {
    fn observe<P: Platform>(board: &P) -> Self {
        Self {
            powered: board.externally_powered(),
            serial: board.logging_connected(),
        }
    }
    fn blocks_sleep(self) -> bool {
        usb_blocks_sleep(self.powered, self.serial, ALLOW_SLEEP_WITH_SERIAL_CONNECTED)
    }
    /// Serial-connected sleep is independent of diagnostic logging.
    fn allows_low_power(self) -> bool {
        !self.serial || ALLOW_SLEEP_WITH_SERIAL_CONNECTED
    }
}

/// Readings derived from one complete motion sample window.
struct Snapshot {
    now_ms: u64,
    stats: MotionStats,
    recent_peak: f32,
    battery: BatteryStatus,
    link: Link,
}

impl Snapshot {
    fn charge_percent(&self) -> Option<u8> {
        ((self.link.powered || self.battery.charging_indicated()) && self.battery.connected)
            .then_some(self.battery.charge_percent)
    }
}

/// The framebuffer plus what the panel is known to show. A view only counts
/// as shown after its transfer succeeds, so failed updates are retried.
struct Screen<'a> {
    frame: FrameBuffer<'a>,
    /// Timer view confirmed on the panel; None forces the next redraw.
    shown_timer_view: Option<TimerView>,
    /// The panel may not match the framebuffer; the next transfer is full.
    full_refresh_pending: bool,
    consecutive_failures: u32,
}

impl<'a> Screen<'a> {
    fn new(frame: FrameBuffer<'a>) -> Self {
        Self {
            frame,
            shown_timer_view: None,
            full_refresh_pending: true,
            consecutive_failures: 0,
        }
    }

    fn invalidate(&mut self) {
        self.shown_timer_view = None;
        self.full_refresh_pending = true;
    }

    /// Replaces the framebuffer with the static layout of `mode`.
    fn draw_mode_frame(&mut self, mode: UiMode) {
        self.frame.clear(Rgb565::BLACK);
        match mode {
            UiMode::Timer => draw_timer_frame(&mut self.frame),
            UiMode::Debug => draw_debug_frame(&mut self.frame),
        }
        self.invalidate();
    }

    /// Sends `regions` of the framebuffer, or all of it while the panel may be
    /// stale. Returns whether the transfer succeeded.
    fn present<P: Platform>(&mut self, board: &mut P, regions: Option<&[Region]>) -> bool {
        let regions = regions.filter(|_| !self.full_refresh_pending);
        match board.show(self.frame.get_buffer(), regions) {
            Ok(()) => {
                self.full_refresh_pending &= regions.is_some();
                self.consecutive_failures = 0;
                true
            }
            Err(error) => {
                self.invalidate();
                self.consecutive_failures = self.consecutive_failures.saturating_add(1);
                write_display_log(
                    board,
                    "error",
                    format_args!(
                        "stage=show error={error:?} count={}",
                        self.consecutive_failures
                    ),
                );
                if self
                    .consecutive_failures
                    .is_multiple_of(DISPLAY_REINIT_FAILURES)
                {
                    let result = board.initialize_display();
                    write_display_log(board, "reinit", format_args!("result={result:?}"));
                }
                false
            }
        }
    }
}

struct App<'a, P: Platform> {
    board: P,
    screen: Screen<'a>,
    imu_address: u8,
    imu_errors: u32,
    shot_timer: ShotTimer,
    previous_state: ShotState,
    battery_monitor: BatteryMonitor,
    /// Last successfully measured status, reused when an ADC read fails.
    last_battery: Option<BatteryStatus>,
    battery_errors: u32,
    recent_peaks: PeakWindow<RECENT_PEAK_WINDOWS>,
    mode_switch: FlipModeSwitch,
    power: Power,
    last_serial_log_ms: u64,
    last_sleep_log_ms: u64,
}

impl<'a, P: Platform> App<'a, P> {
    /// Brings up the LCD and IMU, runs the color test and orientation
    /// calibration, then shows the initial UI.
    fn boot(mut board: P, frame: FrameBuffer<'a>) -> Self {
        let mut screen = Screen::new(frame);
        initialize_display(&mut board);
        board.backlight(true);
        let imu_address = match board.initialize_imu() {
            Ok(address) => address,
            Err(_) => imu_fault(board, screen),
        };
        if COLOR_TEST_ENABLED {
            for color in [Rgb565::RED, Rgb565::GREEN, Rgb565::BLUE] {
                screen.frame.clear(color);
                screen.present(&mut board, None);
                board.delay_ms(1_000);
            }
        }

        let mut imu_errors = 0u32;
        let screen_up_mean = calibrate(&mut board, &mut screen, &mut imu_errors);
        let initial_mode = if DEBUG_MODE_ENABLED && START_IN_DEBUG_MODE {
            UiMode::Debug
        } else {
            UiMode::Timer
        };
        screen.draw_mode_frame(initial_mode);
        screen.present(&mut board, None);
        Self::new(
            board,
            screen,
            imu_address,
            imu_errors,
            FlipModeSwitch::calibrated(initial_mode, screen_up_mean),
        )
    }

    fn new(
        board: P,
        screen: Screen<'a>,
        imu_address: u8,
        imu_errors: u32,
        mode_switch: FlipModeSwitch,
    ) -> Self {
        let shot_timer = ShotTimer::new(board.now_ms());
        Self {
            board,
            screen,
            imu_address,
            imu_errors,
            previous_state: shot_timer.state(),
            shot_timer,
            battery_monitor: BatteryMonitor::new(),
            last_battery: None,
            battery_errors: 0,
            recent_peaks: PeakWindow::new(),
            mode_switch,
            power: Power::Active,
            last_serial_log_ms: 0,
            last_sleep_log_ms: 0,
        }
    }

    fn cycle(&mut self) {
        match self.power {
            Power::Active => self.active_cycle(),
            Power::Sleeping | Power::Restoring => self.sleeping_cycle(),
        }
    }

    fn active_cycle(&mut self) {
        let Some(samples) = self.sample_window() else {
            write_sleep_log(
                &mut self.board,
                "error",
                format_args!("stage=sample count={}", self.imu_errors),
            );
            draw_read_error(&mut self.screen.frame, self.imu_errors);
            self.screen
                .present(&mut self.board, Some(&[READ_ERROR_REGION]));
            // The banner overlays the timer view, so redraw it once reads recover.
            self.screen.shown_timer_view = None;
            return;
        };
        let snapshot = self.observe(&samples);
        let mode_change = self
            .mode_switch
            .update(snapshot.now_ms, snapshot.stats.mean);
        self.log_orientation(&snapshot);
        if let Some(mode) = mode_change {
            self.screen.draw_mode_frame(mode);
        }

        let state = if mode_change.is_some() {
            self.shot_timer.reset(snapshot.now_ms)
        } else {
            self.shot_timer.update_with_sleep_policy(
                snapshot.now_ms,
                snapshot.stats.is_vibrating(VIBRATION_SENSITIVITY_THRESHOLD),
                SLEEP_ENABLED && !snapshot.link.blocks_sleep(),
            )
        };
        self.log_status(&snapshot, state);
        self.apply_state_change(&snapshot, state);
        if !state.is_sleeping() {
            self.render(&snapshot, state);
        }
    }

    /// Reads one motion window; None if any IMU read failed.
    fn sample_window(&mut self) -> Option<[[f32; 3]; SAMPLE_COUNT]> {
        let mut samples = [[0.0; 3]; SAMPLE_COUNT];
        let mut read_ok = true;
        for sample in &mut samples {
            self.board.poll();
            match self.board.read_accel_raw() {
                Ok(value) => {
                    *sample = value.map(|count| count as f32 * METERS_PER_SECOND_SQUARED_PER_COUNT);
                }
                Err(_) => {
                    read_ok = false;
                    self.imu_errors = self.imu_errors.saturating_add(1);
                }
            }
            self.board.delay_ms(SAMPLE_DELAY_MS);
        }
        read_ok.then_some(samples)
    }

    fn observe(&mut self, samples: &[[f32; 3]; SAMPLE_COUNT]) -> Snapshot {
        let stats = MotionStats::from_samples(samples);
        let recent_peak = self.recent_peaks.push(stats.peak_deviation);
        let now_ms = self.board.now_ms();
        let battery = self.sample_battery(now_ms);
        Snapshot {
            now_ms,
            stats,
            recent_peak,
            battery,
            link: Link::observe(&self.board),
        }
    }

    /// Failed ADC reads are counted and keep the last good status instead of
    /// feeding a fabricated 0 V sample into the filter. Before the first
    /// successful read the battery reports as disconnected (no percentage).
    fn sample_battery(&mut self, now_ms: u64) -> BatteryStatus {
        if !USE_BATTERY {
            return self.battery_monitor.update_voltage(0, 0.0, now_ms);
        }
        match self.board.read_battery() {
            Ok(reading) => {
                let status = self.battery_monitor.update_voltage(
                    reading.raw_counts,
                    reading.voltage,
                    now_ms,
                );
                self.last_battery = Some(status);
                status
            }
            Err(_) => {
                self.battery_errors = self.battery_errors.saturating_add(1);
                self.last_battery
                    .unwrap_or_else(|| BatteryStatus::from_adc_counts(0))
            }
        }
    }

    /// Switches the backlight and enters low power when the shot state changes.
    fn apply_state_change(&mut self, snapshot: &Snapshot, state: ShotState) {
        let link = snapshot.link;
        if state == self.previous_state && !(state.is_sleeping() && link.allows_low_power()) {
            return;
        }
        if state.is_sleeping() {
            self.board.backlight(false);
            if link.allows_low_power() {
                self.enter_sleep(snapshot);
            }
        } else {
            self.board.backlight(true);
        }
        self.previous_state = state;
    }

    fn enter_sleep(&mut self, snapshot: &Snapshot) {
        write_sleep_log(
            &mut self.board,
            "enter",
            format_args!(
                "idle_ms={} usb={} serial={}",
                self.shot_timer.idle_ms(snapshot.now_ms),
                snapshot.link.powered,
                snapshot.link.serial
            ),
        );
        let Err(error) = self.board.enter_sleep() else {
            write_sleep_log(
                &mut self.board,
                "entered",
                format_args!("imu_errors={}", self.imu_errors),
            );
            self.power = Power::Sleeping;
            return;
        };
        self.record_sleep_error("enter", error);
        // Undo a partial sleep entry and stay active.
        match self.board.exit_sleep() {
            Ok(cleanup) => {
                if let Some(error) = cleanup {
                    self.record_sleep_error("recover-cleanup", error);
                }
                self.shot_timer.reset(self.board.now_ms());
                self.screen.invalidate();
            }
            Err(error) => {
                self.record_sleep_error("recover", error);
                self.power = Power::Restoring;
            }
        }
    }

    fn sleeping_cycle(&mut self) {
        // No LCD writes or normal sample windows asleep.
        self.board.poll();
        if self.power == Power::Sleeping && !self.wake_requested() {
            self.board.wait_for_wake();
            return;
        }
        match self.board.exit_sleep() {
            Ok(cleanup) => {
                // Sampling and the LCD are restored; a failed WoM disable is
                // only logged and must not keep the board asleep.
                if let Some(error) = cleanup {
                    self.record_sleep_error("exit-cleanup", error);
                }
                write_sleep_log(
                    &mut self.board,
                    "awake",
                    format_args!("imu_errors={}", self.imu_errors),
                );
                self.resume();
            }
            Err(error) => {
                self.record_sleep_error("exit", error);
                self.power = Power::Restoring;
                // Retry after a low-power wait rather than a busy delay, so a
                // dead peripheral does not keep the MCU awake and drain the battery.
                self.board.wait_for_wake();
            }
        }
    }

    /// Checks wake sources. Battery measurements never prevent sleep or wake
    /// the board.
    fn wake_requested(&mut self) -> bool {
        let link = Link::observe(&self.board);
        let usb_wake = link.blocks_sleep();
        let serial_wake = link.serial && !ALLOW_SLEEP_WITH_SERIAL_CONNECTED;
        let (irq, motion, read_error) = match self.board.wake_status() {
            Ok(status) => (Some(status.interrupt_high), status.motion_detected, false),
            Err(error) => {
                self.record_sleep_error("wake-status", error);
                (None, false, true)
            }
        };
        let now_ms = self.board.now_ms();
        if now_ms.saturating_sub(self.last_sleep_log_ms) >= USB_LOG_INTERVAL_MS {
            self.last_sleep_log_ms = now_ms;
            write_sleep_log(
                &mut self.board,
                "check",
                format_args!(
                    "irq={irq:?} wom={motion} imu_error={read_error} usb={} serial={} usb_wake={usb_wake} serial_wake={serial_wake}",
                    link.powered, link.serial
                ),
            );
        }
        // The GPIO level is diagnostic only: an idle/high or stale IRQ is
        // not proof of motion. Reading WoM status confirms and clears the
        // sensor event; timer/GPIO interrupts only prompt another check.
        if !motion && !read_error && !serial_wake && !usb_wake {
            return false;
        }
        write_sleep_log(
            &mut self.board,
            "wake",
            format_args!(
                "irq={irq:?} wom={motion} imu_error={read_error} usb={usb_wake} serial={serial_wake}"
            ),
        );
        true
    }

    /// Restarts active sampling after the IMU and LCD were restored.
    fn resume(&mut self) {
        let now_ms = self.board.now_ms();
        self.shot_timer.reset(now_ms);
        self.previous_state = self.shot_timer.state();
        self.recent_peaks = PeakWindow::new();
        // Replace the retained framebuffer while the backlight is still
        // off; otherwise waking briefly exposes the pre-sleep value.
        draw_wake_frame(
            &mut self.screen.frame,
            self.mode_switch.mode(),
            &self.shot_timer,
            now_ms,
        );
        self.screen.invalidate();
        self.screen.present(&mut self.board, None);
        self.board.backlight(true);
        self.power = Power::Active;
    }

    fn render(&mut self, snapshot: &Snapshot, state: ShotState) {
        let now_ms = snapshot.now_ms;
        match self.mode_switch.mode() {
            UiMode::Debug => {
                let mean = snapshot.stats.mean;
                draw_values(
                    &mut self.screen.frame,
                    self.imu_address,
                    &snapshot.stats,
                    snapshot.recent_peak,
                    DebugStatus {
                        battery: snapshot.battery,
                        shot_state: state,
                        now_ms,
                        idle_ms: self.shot_timer.idle_ms(now_ms),
                        screen_direction: self.mode_switch.screen_direction(mean),
                        screen_vertical: self.mode_switch.screen_vertical(mean),
                        screen_axis: self.mode_switch.screen_axis(),
                        imu_errors: self.imu_errors,
                    },
                );
                self.screen.present(&mut self.board, Some(&DEBUG_REGIONS));
            }
            UiMode::Timer => {
                let view = TimerView::new(
                    state,
                    now_ms,
                    self.shot_timer.previous_shots(),
                    snapshot.charge_percent(),
                );
                if self.screen.shown_timer_view != Some(view) {
                    draw_timer_view(&mut self.screen.frame, view);
                    if self.screen.present(&mut self.board, Some(&TIMER_REGIONS)) {
                        self.screen.shown_timer_view = Some(view);
                    }
                }
            }
        }
    }

    fn record_sleep_error(&mut self, stage: &str, error: crate::HardwareError) {
        self.imu_errors = self.imu_errors.saturating_add(1);
        write_sleep_log(
            &mut self.board,
            "error",
            format_args!("stage={stage} error={error:?} count={}", self.imu_errors),
        );
    }

    fn log_orientation(&mut self, snapshot: &Snapshot) {
        if !USB_LOGGING_ENABLED
            || snapshot.now_ms.saturating_sub(self.last_serial_log_ms) < USB_LOG_INTERVAL_MS
        {
            return;
        }
        self.last_serial_log_ms = snapshot.now_ms;
        let mean = snapshot.stats.mean;
        write_orientation_log(
            &mut self.board,
            &snapshot.stats,
            self.mode_switch.screen_axis(),
            self.mode_switch.screen_raw_vertical(mean),
            self.mode_switch.screen_vertical(mean),
            self.mode_switch.screen_direction(mean),
        );
    }

    fn log_status(&mut self, snapshot: &Snapshot, state: ShotState) {
        let now_ms = snapshot.now_ms;
        if now_ms.saturating_sub(self.last_sleep_log_ms) < USB_LOG_INTERVAL_MS {
            return;
        }
        self.last_sleep_log_ms = now_ms;
        let stats = &snapshot.stats;
        write_sleep_log(
            &mut self.board,
            "status",
            format_args!(
                "state={} idle_ms={} enabled={SLEEP_ENABLED} usb={} blocked_usb={} serial={} sd={:.3} vib={} imu_errors={} battery_errors={}",
                state_label(state),
                self.shot_timer.idle_ms(now_ms),
                snapshot.link.powered,
                snapshot.link.blocks_sleep(),
                snapshot.link.serial,
                stats.peak_deviation,
                stats.is_vibrating(VIBRATION_SENSITIVITY_THRESHOLD),
                self.imu_errors,
                self.battery_errors
            ),
        );
    }
}

/// Retries LCD bring-up instead of halting. The blinking backlight shows the
/// fault and USB serial stays serviced for diagnostics.
fn initialize_display<P: Platform>(board: &mut P) {
    let mut attempts = 0u32;
    while let Err(error) = board.initialize_display() {
        attempts = attempts.saturating_add(1);
        write_display_log(
            board,
            "error",
            format_args!("stage=init error={error:?} count={attempts}"),
        );
        board.backlight(attempts % 2 == 1);
        for _ in 0..DISPLAY_INIT_RETRY_MS / 100 {
            board.poll();
            board.delay_ms(100);
        }
    }
}

/// Without an IMU there is nothing to time: keep the error visible and USB
/// serial serviced.
fn imu_fault<P: Platform>(mut board: P, mut screen: Screen<'_>) -> ! {
    draw_imu_error(&mut screen.frame);
    let mut shown = screen.present(&mut board, Some(&[IMU_ERROR_REGION]));
    loop {
        board.poll();
        board.delay_ms(1000);
        if !shown {
            shown = screen.present(&mut board, Some(&[IMU_ERROR_REGION]));
        }
    }
}

/// Averages screen-up acceleration while showing progress. A window with too
/// many failed reads or a mean far from one gravity is shown, logged, and
/// repeated rather than replaced by a guessed axis.
fn calibrate<P: Platform>(
    board: &mut P,
    screen: &mut Screen<'_>,
    imu_errors: &mut u32,
) -> [f32; 3] {
    loop {
        let calibration = calibration_window(board, screen, imu_errors);
        let error = match calibration.validate(CALIBRATION_MIN_SAMPLES) {
            Ok(mean) => return mean,
            Err(error) => error,
        };
        write_event(
            board,
            "calibration",
            "rejected",
            format_args!("reason={error:?} imu_errors={imu_errors}"),
        );
        screen.frame.clear(Rgb565::BLACK);
        draw_calibration_error(&mut screen.frame, error);
        screen.present(board, None);
        for _ in 0..CALIBRATION_RETRY_MS / 100 {
            board.poll();
            board.delay_ms(100);
        }
    }
}

fn calibration_window<P: Platform>(
    board: &mut P,
    screen: &mut Screen<'_>,
    imu_errors: &mut u32,
) -> OrientationCalibration {
    let mut calibration = OrientationCalibration::new();
    screen.frame.clear(Rgb565::BLACK);
    draw_calibrating(&mut screen.frame, [0.0; 3], calibration.sample_count());
    screen.present(board, None);
    for attempt in 1..=CALIBRATION_SAMPLES {
        board.poll();
        match board.read_accel_raw() {
            Ok(value) => {
                calibration
                    .add(value.map(|count| count as f32 * METERS_PER_SECOND_SQUARED_PER_COUNT));
            }
            Err(_) => *imu_errors = imu_errors.saturating_add(1),
        }
        board.delay_ms(SAMPLE_DELAY_MS);

        if attempt % CALIBRATION_DISPLAY_INTERVAL_SAMPLES == 0 {
            screen.frame.clear(Rgb565::BLACK);
            draw_calibrating(
                &mut screen.frame,
                calibration.mean().unwrap_or([0.0; 3]),
                calibration.sample_count(),
            );
            screen.present(board, None);
        }
    }
    calibration
}

fn state_label(state: ShotState) -> &'static str {
    match state {
        ShotState::Ready => "ready",
        ShotState::Confirming { .. } => "confirming",
        ShotState::Timing { .. } => "timing",
        ShotState::Completed { .. } => "completed",
        ShotState::RestartConfirming { .. } => "restarting",
        ShotState::TimedOut => "timeout",
        ShotState::Sleeping => "sleeping",
    }
}

fn draw_wake_frame<D>(display: &mut D, mode: UiMode, timer: &ShotTimer, now_ms: u64)
where
    D: DrawTarget<Color = Rgb565>,
{
    display.clear(Rgb565::BLACK).ok();
    match mode {
        UiMode::Timer => draw_timer_view(
            display,
            TimerView::new(timer.state(), now_ms, timer.previous_shots(), None),
        ),
        // Fresh sensor/battery readings are drawn after the next sample window.
        UiMode::Debug => draw_debug_frame(display),
    }
}

fn draw_calibrating<D>(display: &mut D, acceleration: [f32; 3], samples: u32)
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

fn draw_calibration_error<D>(display: &mut D, error: CalibrationError)
where
    D: DrawTarget<Color = Rgb565>,
{
    let style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
    let mut detail = String::<32>::new();
    match error {
        CalibrationError::TooFewSamples { samples, required } => {
            write!(detail, "IMU READS {samples}/{required}")
        }
        CalibrationError::NotGravity { magnitude } => write!(detail, "ACCEL {magnitude:.2} m/s2"),
    }
    .ok();
    for (text, y) in [
        ("CALIBRATION FAILED", 95),
        (detail.as_str(), 120),
        ("RETRYING", 145),
    ] {
        Text::with_alignment(text, Point::new(120, y), style, Alignment::Center)
            .draw(display)
            .ok();
    }
}

fn usb_blocks_sleep(powered: bool, serial: bool, allow_serial_sleep: bool) -> bool {
    powered && !(serial && allow_serial_sleep)
}

fn write_sleep_log<P: Platform>(board: &mut P, event: &str, details: core::fmt::Arguments<'_>) {
    if SLEEP_DIAGNOSTICS_ENABLED {
        write_event(board, "sleep", event, details);
    }
}

fn write_display_log<P: Platform>(board: &mut P, event: &str, details: core::fmt::Arguments<'_>) {
    write_event(board, "display", event, details);
}

fn write_event<P: Platform>(
    board: &mut P,
    topic: &str,
    event: &str,
    details: core::fmt::Arguments<'_>,
) {
    if !USB_LOGGING_ENABLED {
        return;
    }
    let mut line = String::<256>::new();
    writeln!(
        line,
        "{topic} t={} event={event} {details}\r",
        board.now_ms()
    )
    .ok();
    board.log(line.as_bytes());
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
    use shottimer_core::ui_mode::STANDARD_GRAVITY;
    use std::{
        cell::RefCell,
        collections::VecDeque,
        panic::{AssertUnwindSafe, catch_unwind},
        rc::Rc,
        vec,
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
        exit_attempts: Vec<u64>,
        waits: usize,
        display_inits: usize,
        /// Region count per transfer; None is a full-frame transfer.
        shows: Vec<Option<usize>>,
        backlight: bool,
        events: Vec<std::string::String>,
    }
    struct FakeBoard {
        now: u64,
        trace: Rc<RefCell<Trace>>,
        display_ready: bool,
        /// Treat the first full transfers as the boot RGB test.
        color_test_pending: bool,
        asleep: bool,
        display_init_failures: usize,
        show_failures: usize,
        exit_results: VecDeque<Result<Option<HardwareError>, HardwareError>>,
        motion_from_ms: u64,
        /// IMU reads fail before this uptime.
        reads_fail_until_ms: u64,
        battery_fails: bool,
    }
    impl FakeBoard {
        fn new(now: u64, trace: Rc<RefCell<Trace>>) -> Self {
            Self {
                now,
                trace,
                display_ready: false,
                color_test_pending: COLOR_TEST_ENABLED,
                asleep: false,
                display_init_failures: 0,
                show_failures: 0,
                exit_results: VecDeque::new(),
                motion_from_ms: 80_000,
                reads_fail_until_ms: 0,
                battery_fails: false,
            }
        }
        /// The state boot leaves behind: LCD initialized, RGB test done.
        fn booted(now: u64, trace: Rc<RefCell<Trace>>) -> Self {
            Self {
                display_ready: true,
                color_test_pending: false,
                ..Self::new(now, trace)
            }
        }
    }
    impl Platform for FakeBoard {
        fn now_ms(&self) -> u64 {
            self.now
        }
        fn delay_ms(&mut self, ms: u32) {
            self.now += u64::from(ms);
        }
        fn initialize_display(&mut self) -> Result<(), HardwareError> {
            self.trace.borrow_mut().display_inits += 1;
            if self.display_init_failures > 0 {
                self.display_init_failures -= 1;
                return Err(HardwareError::Display);
            }
            self.display_ready = true;
            Ok(())
        }
        fn initialize_imu(&mut self) -> Result<u8, HardwareError> {
            Ok(0x6b)
        }
        fn read_accel_raw(&mut self) -> Result<[i16; 3], HardwareError> {
            let mut trace = self.trace.borrow_mut();
            trace.reads += 1;
            if self.now < self.reads_fail_until_ms {
                return Err(HardwareError::Imu);
            }
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
            assert!(!self.asleep, "battery must not be sampled asleep");
            self.trace.borrow_mut().battery_reads += 1;
            if self.battery_fails {
                return Err(HardwareError::Battery);
            }
            Ok(BatteryReading {
                raw_counts: 2400,
                // Sustained rising voltage must not block idle sleep or wake it.
                voltage: 3.85 + self.now as f32 * 0.000_000_1,
            })
        }
        fn show(&mut self, buf: &[u8], regions: Option<&[Region]>) -> Result<(), HardwareError> {
            assert!(self.display_ready, "LCD written before initialization");
            assert!(!self.asleep, "LCD written while asleep");
            let mut trace = self.trace.borrow_mut();
            trace.shows.push(regions.map(<[Region]>::len));
            if self.show_failures > 0 {
                self.show_failures -= 1;
                return Err(HardwareError::Display);
            }
            if self.color_test_pending && regions.is_none() && trace.colors.len() < 3 {
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
        fn backlight(&mut self, enabled: bool) {
            self.trace.borrow_mut().backlight = enabled;
        }
        fn poll(&mut self) {
            if self.now >= 90_000 {
                std::panic::panic_any("simulation complete");
            }
        }
        fn externally_powered(&self) -> bool {
            ALLOW_SLEEP_WITH_SERIAL_CONNECTED
        }
        fn logging_connected(&self) -> bool {
            ALLOW_SLEEP_WITH_SERIAL_CONNECTED
        }
        fn log(&mut self, bytes: &[u8]) {
            let mut trace = self.trace.borrow_mut();
            trace.logs += 1;
            if !bytes.starts_with(b"imu x=") {
                let line = std::str::from_utf8(bytes).unwrap();
                assert!(
                    ["sleep t=", "display t=", "calibration t="]
                        .iter()
                        .any(|prefix| line.starts_with(prefix))
                );
                trace.events.push(line.into());
            }
        }
        fn enter_sleep(&mut self) -> Result<(), HardwareError> {
            self.trace.borrow_mut().sleeps += 1;
            self.asleep = true;
            Ok(())
        }
        fn wake_status(&mut self) -> Result<crate::WakeStatus, HardwareError> {
            Ok(crate::WakeStatus {
                // Reproduce the hardware's idle-high pin with no WoM event.
                interrupt_high: true,
                motion_detected: self.now >= self.motion_from_ms,
            })
        }
        fn wait_for_wake(&mut self) {
            self.trace.borrow_mut().waits += 1;
            self.now += 500;
        }
        fn exit_sleep(&mut self) -> Result<Option<HardwareError>, HardwareError> {
            let mut trace = self.trace.borrow_mut();
            trace.exit_attempts.push(self.now);
            let result = self.exit_results.pop_front().unwrap_or(Ok(None));
            if result.is_ok() {
                trace.wakes += 1;
                self.asleep = false;
            }
            result
        }
    }

    fn calibrated_switch() -> FlipModeSwitch {
        FlipModeSwitch::calibrated(UiMode::Timer, [0.0, 0.0, STANDARD_GRAVITY])
    }

    fn has_event(trace: &Trace, needle: &str) -> bool {
        trace.events.iter().any(|line| line.contains(needle))
    }

    #[test]
    fn shared_loop_runs_boot_calibration_sampling_logging_and_sleep_recovery() {
        let trace = Rc::new(RefCell::new(Trace::default()));
        let result = catch_unwind(AssertUnwindSafe(|| run(FakeBoard::new(0, trace.clone()))));
        let reason = result.unwrap_err();
        assert_eq!(reason.downcast_ref::<&str>(), Some(&"simulation complete"));
        let trace = trace.borrow();
        if COLOR_TEST_ENABLED {
            assert_eq!(trace.colors, [0xf800, 0x07e0, 0x001f]);
        } else {
            assert!(trace.colors.is_empty());
        }
        assert_eq!(trace.display_inits, 1);
        assert!(trace.reads >= 300);
        assert_eq!(trace.battery_reads > 0, USE_BATTERY);
        assert!(trace.logs > 100);
        assert_eq!(trace.sleeps, usize::from(SLEEP_ENABLED));
        assert_eq!(trace.wakes, usize::from(SLEEP_ENABLED));
        assert_eq!(trace.waits > 0, SLEEP_ENABLED);
        assert!(
            trace.exit_attempts.iter().all(|&ms| ms >= 80_000),
            "idle-high IRQ must not wake the application"
        );
        if USB_LOGGING_ENABLED && SLEEP_DIAGNOSTICS_ENABLED && SLEEP_ENABLED {
            assert!(has_event(&trace, "irq=Some(true) wom=false"));
            for event in [
                "event=status",
                "event=enter ",
                "event=entered",
                "event=check",
                "event=wake ",
                "event=awake",
            ] {
                assert!(has_event(&trace, event), "missing {event}");
            }
            assert!(has_event(
                &trace,
                "irq=Some(true) wom=true imu_error=false usb=false serial=false"
            ));
        }
    }

    #[test]
    fn display_initialization_failure_is_retried_before_boot_continues() {
        let trace = Rc::new(RefCell::new(Trace::default()));
        let mut board = FakeBoard::new(0, trace.clone());
        board.display_init_failures = 2;
        let mut pixels = vec![0; LCD_BUFFER_BYTES];
        // FakeBoard::show panics if the LCD is written before initialization.
        let app = App::boot(board, FrameBuffer::new(&mut pixels, LCD_SIZE, LCD_SIZE));
        assert_eq!(app.power, Power::Active);
        let trace = trace.borrow();
        assert_eq!(trace.display_inits, 3);
        assert!(trace.backlight);
        if USB_LOGGING_ENABLED {
            assert!(has_event(&trace, "stage=init error=Display count=2"));
        }
    }

    #[test]
    fn calibration_with_failed_reads_is_repeated_instead_of_guessing_an_axis() {
        let trace = Rc::new(RefCell::new(Trace::default()));
        let mut board = FakeBoard::new(0, trace.clone());
        // Most of the first window (after the 3 s RGB test) fails to read.
        board.reads_fail_until_ms = 5_500;
        let mut pixels = vec![0; LCD_BUFFER_BYTES];
        let app = App::boot(board, FrameBuffer::new(&mut pixels, LCD_SIZE, LCD_SIZE));
        assert_eq!(app.mode_switch.screen_axis(), ScreenAxis::Z);
        assert!(app.imu_errors > CALIBRATION_MIN_SAMPLES);
        assert!(app.board.now >= 3_000 + 2 * CALIBRATION_DURATION_MS as u64);
        if USB_LOGGING_ENABLED {
            assert!(has_event(
                &trace.borrow(),
                "event=rejected reason=TooFewSamples"
            ));
        }
    }

    #[test]
    fn failed_transfer_forces_full_refresh_and_repeated_failures_reinitialize() {
        let trace = Rc::new(RefCell::new(Trace::default()));
        let mut board = FakeBoard::booted(0, trace.clone());
        let mut pixels = vec![0; LCD_BUFFER_BYTES];
        let mut screen = Screen::new(FrameBuffer::new(&mut pixels, LCD_SIZE, LCD_SIZE));
        let regions = Some(&TIMER_REGIONS[..]);

        // The panel content is unknown at first, so partial updates are sent in full.
        assert!(screen.present(&mut board, regions));
        assert!(screen.present(&mut board, regions));
        assert_eq!(trace.borrow().shows, [None, Some(1)]);

        trace.borrow_mut().shows.clear();
        board.show_failures = DISPLAY_REINIT_FAILURES as usize;
        screen.shown_timer_view = Some(TimerView::new(ShotState::Ready, 0, [None; 3], None));
        for _ in 0..DISPLAY_REINIT_FAILURES {
            assert!(!screen.present(&mut board, regions));
            assert_eq!(screen.shown_timer_view, None);
        }
        assert_eq!(trace.borrow().display_inits, 1);
        assert!(screen.present(&mut board, regions));
        assert!(screen.present(&mut board, regions));
        assert_eq!(trace.borrow().shows, [Some(1), None, None, None, Some(1)]);
    }

    #[test]
    fn timer_view_is_not_cached_until_its_transfer_succeeds() {
        let trace = Rc::new(RefCell::new(Trace::default()));
        let mut board = FakeBoard::booted(20_000, trace.clone());
        board.show_failures = 1;
        let mut pixels = vec![0; LCD_BUFFER_BYTES];
        let screen = Screen::new(FrameBuffer::new(&mut pixels, LCD_SIZE, LCD_SIZE));
        let mut app = App::new(board, screen, 0x6b, 0, calibrated_switch());
        app.cycle();
        assert_eq!(app.screen.shown_timer_view, None);
        app.cycle();
        assert!(app.screen.shown_timer_view.is_some());
        app.cycle();
        // Failed, retried in full, then unchanged views send nothing.
        assert_eq!(trace.borrow().shows, [None, None]);
    }

    #[test]
    fn timer_view_is_redrawn_after_a_read_error_banner() {
        let trace = Rc::new(RefCell::new(Trace::default()));
        let board = FakeBoard::booted(20_000, trace.clone());
        let mut pixels = vec![0; LCD_BUFFER_BYTES];
        let screen = Screen::new(FrameBuffer::new(&mut pixels, LCD_SIZE, LCD_SIZE));
        let mut app = App::new(board, screen, 0x6b, 0, calibrated_switch());
        app.cycle();
        assert!(app.screen.shown_timer_view.is_some());
        app.board.reads_fail_until_ms = app.board.now + 50;
        app.cycle();
        app.cycle();
        // Initial view, error banner, then the unchanged view over the banner.
        assert_eq!(trace.borrow().shows, [None, Some(1), Some(1)]);
    }

    #[test]
    fn failed_wake_keeps_restoring_without_another_motion_event() {
        let trace = Rc::new(RefCell::new(Trace::default()));
        let mut board = FakeBoard::booted(20_000, trace.clone());
        board.asleep = true;
        board.motion_from_ms = 0;
        board.exit_results = [Err(HardwareError::Imu), Err(HardwareError::Display)].into();
        let mut pixels = vec![0; LCD_BUFFER_BYTES];
        let screen = Screen::new(FrameBuffer::new(&mut pixels, LCD_SIZE, LCD_SIZE));
        let mut app = App::new(board, screen, 0x6b, 0, calibrated_switch());
        app.power = Power::Sleeping;

        app.cycle();
        assert_eq!(app.power, Power::Restoring);
        // The motion latch was consumed; restoration must not wait for more.
        app.board.motion_from_ms = u64::MAX;
        app.cycle();
        assert_eq!(app.power, Power::Restoring);
        app.cycle();
        assert_eq!(app.power, Power::Active);
        assert_eq!(app.imu_errors, 2);
        let trace = trace.borrow();
        assert_eq!(trace.exit_attempts.len(), 3);
        // Each failed attempt waits in low power before retrying.
        assert_eq!(trace.waits, 2);
        assert_eq!(trace.shows, [None]);
        assert!(trace.backlight);
    }

    #[test]
    fn failed_battery_read_keeps_last_good_status() {
        let trace = Rc::new(RefCell::new(Trace::default()));
        let board = FakeBoard::booted(1_000, trace);
        let mut pixels = vec![0; LCD_BUFFER_BYTES];
        let screen = Screen::new(FrameBuffer::new(&mut pixels, LCD_SIZE, LCD_SIZE));
        let mut app = App::new(board, screen, 0x6b, 0, calibrated_switch());
        let samples = [[0.0, 0.0, STANDARD_GRAVITY]; SAMPLE_COUNT];

        app.board.battery_fails = true;
        let before_first_read = app.observe(&samples).battery;
        assert!(!before_first_read.connected);

        app.board.battery_fails = false;
        let good = app.observe(&samples).battery;
        app.board.battery_fails = true;
        for _ in 0..5 {
            assert_eq!(app.observe(&samples).battery, good);
        }
        assert_eq!(app.battery_errors, if USE_BATTERY { 6 } else { 0 });
        if USE_BATTERY {
            assert!(good.connected);
            // The filter was not dragged toward 0 V by the failures.
            app.board.battery_fails = false;
            assert!(app.observe(&samples).battery.voltage > 3.8);
        }
    }

    #[test]
    fn wom_cleanup_failure_still_resumes_active_sampling() {
        let trace = Rc::new(RefCell::new(Trace::default()));
        let mut board = FakeBoard::booted(80_000, trace.clone());
        board.asleep = true;
        board.exit_results = [Ok(Some(HardwareError::ImuCommandTimeout))].into();
        let mut pixels = vec![0; LCD_BUFFER_BYTES];
        let screen = Screen::new(FrameBuffer::new(&mut pixels, LCD_SIZE, LCD_SIZE));
        let mut app = App::new(board, screen, 0x6b, 0, calibrated_switch());
        app.power = Power::Sleeping;
        app.cycle();
        assert_eq!(app.power, Power::Active);
        assert_eq!(app.imu_errors, 1);
        assert!(trace.borrow().backlight);
        if USB_LOGGING_ENABLED && SLEEP_DIAGNOSTICS_ENABLED {
            assert!(has_event(
                &trace.borrow(),
                "stage=exit-cleanup error=ImuCommandTimeout"
            ));
        }
    }

    #[test]
    fn serial_sleep_override_requires_a_connected_terminal() {
        for allow_serial_sleep in [false, true] {
            assert!(!usb_blocks_sleep(false, false, allow_serial_sleep));
            assert!(!usb_blocks_sleep(false, true, allow_serial_sleep));
            assert!(usb_blocks_sleep(true, false, allow_serial_sleep));
        }
        assert!(usb_blocks_sleep(true, true, false));
        assert!(!usb_blocks_sleep(true, true, true));
    }

    #[test]
    fn wake_frame_replaces_old_timer_and_debug_pixels() {
        let mut pixels = [0xff; LCD_BUFFER_BYTES];
        let mut frame = FrameBuffer::new(&mut pixels, LCD_SIZE, LCD_SIZE);
        let mut expected_pixels = [0; LCD_BUFFER_BYTES];
        let mut expected = FrameBuffer::new(&mut expected_pixels, LCD_SIZE, LCD_SIZE);
        let mut timer = ShotTimer::new(0);
        timer.reset(80_000);
        draw_wake_frame(&mut frame, UiMode::Timer, &timer, 80_000);
        draw_timer_view(
            &mut expected,
            TimerView::new(timer.state(), 80_000, timer.previous_shots(), None),
        );
        assert_eq!(frame.get_buffer(), expected.get_buffer());

        frame.clear(Rgb565::WHITE);
        draw_wake_frame(&mut frame, UiMode::Debug, &timer, 80_000);
        expected.clear(Rgb565::BLACK);
        draw_debug_frame(&mut expected);
        assert_eq!(frame.get_buffer(), expected.get_buffer());
    }

    #[test]
    fn sleeping_cycle_ignores_idle_high_irq_then_restores_on_motion() {
        let trace = Rc::new(RefCell::new(Trace::default()));
        let mut board = FakeBoard::booted(75_000, trace.clone());
        board.asleep = true;
        let mut pixels = vec![0; LCD_BUFFER_BYTES];
        let screen = Screen::new(FrameBuffer::new(&mut pixels, LCD_SIZE, LCD_SIZE));
        let mut app = App::new(board, screen, 0x6b, 0, calibrated_switch());
        app.power = Power::Sleeping;
        app.cycle();
        assert_eq!(app.power, Power::Sleeping);
        {
            let trace = trace.borrow();
            assert_eq!(trace.waits, 1);
            assert_eq!(trace.wakes, 0);
            assert_eq!(trace.reads, 0);
            assert_eq!(trace.battery_reads, 0);
        }
        app.board.now = 80_000;
        app.cycle();
        assert_eq!(app.power, Power::Active);
        assert_eq!(trace.borrow().wakes, 1);
        assert_eq!(app.imu_errors, 0);
    }
}
