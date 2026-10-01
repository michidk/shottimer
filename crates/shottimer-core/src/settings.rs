//! Calibration values shared by motion detection and the debug display.

/// Delay between accelerometer readings in one motion window.
pub const SAMPLE_DELAY_MS: u32 = 10;

/// Selects the UI shown after the RGB display test.
pub const START_IN_DEBUG_MODE: bool = false;

/// Show up to three previous completed shot times throughout Timer mode.
pub const SHOW_SHOT_HISTORY: bool = true;

/// Clockwise LCD rotation relative to the current board orientation.
pub const DISPLAY_ROTATION_DEGREES: u16 = 90;

/// Backlight PWM duty cycle, from 0 (off) to 100 (full brightness).
pub const DISPLAY_BRIGHTNESS_PERCENT: u8 = 100;

/// Idle duration before the display backlight is turned off.
pub const SLEEP_TIMEOUT_SECONDS: u64 = 60;

/// Hardware wake-on-motion acceleration change, in mg (not vibration SD).
pub const SLEEP_WAKE_THRESHOLD_MG: u8 = 50;
/// Low-power recovery/USB service interval; GPIO motion wakes immediately.
pub const SLEEP_CHECK_INTERVAL_MS: u32 = 500;

/// Delay before initial vibration is accepted as a shot.
pub const START_CONFIRM_SECONDS: u64 = 2;

/// Nominal IMU calibration sampling duration after the RGB test.
/// Display transfers add a small amount of overhead.
pub const CALIBRATION_DURATION_MS: u32 = 3_000;

/// Emit USB CDC diagnostics when a terminal is connected.
pub const USB_LOGGING_ENABLED: bool = true;
pub const USB_LOG_INTERVAL_MS: u64 = 500;

/// Enable battery monitoring, charge estimates, and battery UI.
/// The charger can power the measured rail over USB without a battery.
pub const USE_BATTERY: bool = false;

/// Battery ADC reference voltage and ADC-input/battery divider ratio.
pub const BATTERY_ADC_REFERENCE_VOLTS: f32 = 3.3;
pub const BATTERY_VOLTAGE_DIVIDER_RATIO: f32 = 0.5;

const _: () = {
    assert!(matches!(DISPLAY_ROTATION_DEGREES, 0 | 90 | 180 | 270));
    assert!(DISPLAY_BRIGHTNESS_PERCENT <= 100);
    assert!(SAMPLE_DELAY_MS > 0);
    assert!(CALIBRATION_DURATION_MS >= SAMPLE_DELAY_MS);
    assert!(CALIBRATION_DURATION_MS.is_multiple_of(SAMPLE_DELAY_MS));
    assert!(SLEEP_TIMEOUT_SECONDS > 0 && START_CONFIRM_SECONDS > 0);
    assert!(SLEEP_WAKE_THRESHOLD_MG > 0 && SLEEP_CHECK_INTERVAL_MS > 0);
    assert!(USB_LOG_INTERVAL_MS > 0);
    assert!(BATTERY_ADC_REFERENCE_VOLTS > 0.0);
    assert!(BATTERY_VOLTAGE_DIVIDER_RATIO > 0.0 && BATTERY_VOLTAGE_DIVIDER_RATIO <= 1.0);
};

/// Peak per-axis standard deviation that counts as vibration, in m/s².
/// Increase this value to reduce sensitivity; decrease it to detect weaker vibration.
pub const VIBRATION_SENSITIVITY_THRESHOLD: f32 = 1.0;

/// Minimum normalized calibrated-axis component accepted as screen-up/down.
/// 0.7 accepts orientations within roughly 46° of directly face-up/down.
pub const SCREEN_VERTICAL_COS_THRESHOLD: f32 = 0.7;

/// Weight of each new 100 ms orientation reading in the low-pass filter.
pub const ORIENTATION_FILTER_ALPHA: f32 = 0.2;

/// Consecutive averaged 100 ms face-down windows required to arm a mode change.
pub const SCREEN_DOWN_DEBOUNCE_WINDOWS: u8 = 10;

/// Consecutive averaged 100 ms face-up windows required to complete a mode change.
pub const SCREEN_RELEASE_DEBOUNCE_WINDOWS: u8 = 3;

/// Maximum displayed shot duration before timeout.
pub const SHOT_TIMEOUT_SECONDS: u64 = 99;

/// Completed shots shorter than this are discarded instead of retained.
pub const MINIMUM_SHOT_SECONDS: u64 = 5;

/// Continuous vibration required to replace a retained result with a new shot.
pub const RESTART_CONFIRM_SECONDS: u64 = 3;

/// Seconds represented by one complete lap of the Timer-mode progress arc.
pub const PROGRESS_LAP_SECONDS: u64 = 25;

/// Ring palette in native RGB565 channel values: red 0..31, green 0..63,
/// blue 0..31. Edge shades soften the outer pixel boundary of each arc.
pub const RING_FIRST_LAP_COLOR: (u8, u8, u8) = (22, 28, 9);
pub const RING_FIRST_LAP_EDGE_COLOR: (u8, u8, u8) = (11, 14, 5);
pub const RING_SECOND_LAP_COLOR: (u8, u8, u8) = (31, 20, 6);
pub const RING_SECOND_LAP_EDGE_COLOR: (u8, u8, u8) = (16, 10, 3);
pub const RING_TRACK_COLOR: (u8, u8, u8) = (0, 10, 8);
pub const RING_TRACK_EDGE_COLOR: (u8, u8, u8) = (0, 5, 4);

/// Seconds to retain a completed shot result.
pub const COMPLETED_SHOT_HOLD_SECONDS: u64 = 60;

/// Number of motion windows retained by the rolling peak indicator.
pub const RECENT_PEAK_WINDOWS: usize = 32;

/// QMI8658 raw acceleration conversion for the configured ±8 g range.
pub const METERS_PER_SECOND_SQUARED_PER_COUNT: f32 = 9.806_65 / 4096.0;
