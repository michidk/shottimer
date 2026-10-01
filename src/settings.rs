//! Calibration values shared by motion detection and the debug display.

/// Delay between accelerometer readings in one motion window.
pub const SAMPLE_DELAY_MS: u32 = 10;

/// Selects the UI shown after the RGB display test.
pub const START_IN_DEBUG_MODE: bool = false;

/// Peak per-axis standard deviation that counts as vibration, in m/s².
/// Increase this value to reduce sensitivity; decrease it to detect weaker vibration.
pub const VIBRATION_SENSITIVITY_THRESHOLD: f32 = 12.0;

/// Minimum normalized calibrated-axis component accepted as screen-up/down.
/// 0.7 accepts orientations within roughly 46° of directly face-up/down.
pub const SCREEN_VERTICAL_COS_THRESHOLD: f32 = 0.7;

/// Weight of each new 100 ms orientation reading in the low-pass filter.
pub const ORIENTATION_FILTER_ALPHA: f32 = 0.2;

/// Consecutive averaged 100 ms face-down windows required to arm a mode change.
pub const SCREEN_DOWN_DEBOUNCE_WINDOWS: u8 = 10;

/// Consecutive averaged 100 ms face-up windows required to complete a mode change.
pub const SCREEN_RELEASE_DEBOUNCE_WINDOWS: u8 = 3;

/// Maximum displayed shot duration and full-scale value for the progress arc.
pub const SHOT_TIMEOUT_SECONDS: u64 = 99;

/// Completed shots shorter than this are discarded instead of retained.
pub const MINIMUM_SHOT_SECONDS: u64 = 5;

/// Continuous vibration required to replace a retained result with a new shot.
pub const RESTART_CONFIRM_SECONDS: u64 = 3;

/// Seconds represented by one complete lap of the Timer-mode progress arc.
pub const PROGRESS_LAP_SECONDS: u64 = 25;

/// Seconds to retain a completed shot result.
pub const COMPLETED_SHOT_HOLD_SECONDS: u64 = 60;

/// Number of motion windows retained by the rolling peak indicator.
pub const RECENT_PEAK_WINDOWS: usize = 32;

/// QMI8658 raw acceleration conversion for the configured ±8 g range.
pub const METERS_PER_SECOND_SQUARED_PER_COUNT: f32 = 9.806_65 / 4096.0;
