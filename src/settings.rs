//! Calibration values shared by motion detection and the debug display.

/// Delay between accelerometer readings in one motion window.
pub const SAMPLE_DELAY_MS: u32 = 10;

/// Peak per-axis standard deviation that counts as vibration, in m/s².
pub const VIBRATION_THRESHOLD: f32 = 10.0;

/// Minimum normalized Z component accepted as screen-up/down.
/// 0.5 accepts orientations within roughly 60° of directly face-down.
pub const SCREEN_VERTICAL_COS_THRESHOLD: f32 = 0.5;

/// Normalized screen-relative Z above which the device is no longer screen-down.
pub const SCREEN_DOWN_RELEASE_COS: f32 = -0.3;

/// Measured sign converting the QMI8658 Z reading to screen-relative Z.
pub const SCREEN_UP_Z_SIGN: f32 = 1.0;

/// Exponential low-pass weight applied to each 100 ms orientation window.
pub const ORIENTATION_FILTER_ALPHA: f32 = 0.4;

/// Consecutive filtered 100 ms windows required to enter Debug mode.
pub const SCREEN_DOWN_DEBOUNCE_WINDOWS: u8 = 10;

/// Consecutive filtered 100 ms windows required to return to Timer mode.
pub const SCREEN_RELEASE_DEBOUNCE_WINDOWS: u8 = 3;

/// Maximum displayed shot duration and full-scale value for the progress arc.
pub const SHOT_TIMEOUT_SECONDS: u64 = 99;

/// Seconds represented by one complete lap of the Timer-mode progress arc.
pub const PROGRESS_LAP_SECONDS: u64 = 25;

/// Seconds to retain a completed shot result.
pub const COMPLETED_SHOT_HOLD_SECONDS: u64 = 60;

/// Number of motion windows retained by the rolling peak indicator.
pub const RECENT_PEAK_WINDOWS: usize = 32;

/// QMI8658 raw acceleration conversion for the configured ±8 g range.
pub const METERS_PER_SECOND_SQUARED_PER_COUNT: f32 = 9.806_65 / 4096.0;
