//! Calibration values shared by motion detection and the debug display.
//!
//! Values come from the board's TOML config set in `configs/`, see `build.rs`.

include!(concat!(env!("OUT_DIR"), "/settings.rs"));

/// QMI8658 raw acceleration conversion for the configured ±8 g range.
pub const METERS_PER_SECOND_SQUARED_PER_COUNT: f32 = 9.806_65 / 4096.0;

const _: () = {
    assert!(matches!(DISPLAY_ROTATION_DEGREES, 0 | 90 | 180 | 270));
    assert!(DISPLAY_BRIGHTNESS_PERCENT <= 100);
    assert!(SAMPLE_DELAY_MS > 0);
    assert!(CALIBRATION_DURATION_MS >= SAMPLE_DELAY_MS);
    assert!(CALIBRATION_DURATION_MS.is_multiple_of(SAMPLE_DELAY_MS));
    assert!(SLEEP_TIMEOUT_SECONDS > 0 && START_CONFIRM_SECONDS > 0);
    assert!(SLEEP_WAKE_THRESHOLD_MG > 0 && SLEEP_CHECK_INTERVAL_MS > 0);
    assert!(USB_LOG_INTERVAL_MS > 0);
    assert!(SCREEN_DOWN_HOLD_MS > 0);
    assert!(VIBRATION_SD_DEADBAND >= 0.0 && VIBRATION_SD_DEADBAND < f32::INFINITY);
    assert!(BATTERY_ADC_REFERENCE_VOLTS > 0.0);
    assert!(BATTERY_VOLTAGE_DIVIDER_RATIO > 0.0 && BATTERY_VOLTAGE_DIVIDER_RATIO <= 1.0);
};
