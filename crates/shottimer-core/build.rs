//! Generates `settings` constants from a TOML config set.
//!
//! The file is chosen by `SHOTTIMER_CONFIG` (relative to the workspace root),
//! else by the enabled `board-*` feature (`configs/<board>.toml`). Host builds
//! without a board feature use the defaults below. Keys missing from the file
//! keep their default; unknown keys and out-of-range values fail the build.

use std::{env, fmt::Write as _, fs, path::PathBuf};

#[derive(Clone, Copy)]
enum Kind {
    Bool,
    U8,
    U16,
    U32,
    U64,
    Usize,
    F32,
    /// `[r, g, b]` in native RGB565 channel ranges, emitted as `(u8, u8, u8)`.
    Rgb565,
    /// `"+X"` .. `"-Z"`, emitted as a unit `[f32; 3]` vector.
    Axis,
}

struct Setting {
    name: &'static str,
    kind: Kind,
    default: &'static str,
    doc: &'static str,
}

const fn setting(
    name: &'static str,
    kind: Kind,
    default: &'static str,
    doc: &'static str,
) -> Setting {
    Setting {
        name,
        kind,
        default,
        doc,
    }
}

use Kind::*;

#[rustfmt::skip]
const SETTINGS: &[Setting] = &[
    // Modes and boot.
    setting("debug_mode_enabled", Bool, "true", "Allow Debug mode at startup and through the mode-switch gesture."),
    setting("start_in_debug_mode", Bool, "false", "Selects the UI shown after the RGB display test."),
    setting("color_test_enabled", Bool, "true", "Show red, green, and blue for one second each before IMU calibration.\nFalse skips only the display test, not calibration."),
    setting("touch_mode_enabled", Bool, "false", "Switch modes with vertical touchscreen swipes instead of the flip gesture\nand skip boot calibration. Falls back to flip mode without a touch controller."),
    setting("swipe_min_distance_px", U16, "60", "Minimum finger travel, in display pixels, recognized as a swipe."),
    setting("screen_up_axis", Axis, "\"+Z\"", "Accelerometer axis pointing out of the screen (`+X`..`-Z`). Replaces boot\ncalibration in touch mode; otherwise unused."),
    // Display.
    setting("show_shot_history", Bool, "true", "Show up to three previous completed shot times throughout Timer mode."),
    setting("display_rotation_degrees", U16, "270", "Clockwise LCD rotation relative to the current board orientation."),
    setting("display_brightness_percent", U8, "100", "Backlight PWM duty cycle, from 0 (off) to 100 (full brightness)."),
    // Motion sampling and vibration.
    setting("sample_delay_ms", U32, "10", "Delay between accelerometer readings in one motion window."),
    setting("vibration_sensitivity_threshold", F32, "1.0", "Peak per-axis standard deviation that counts as vibration, in m/s².\nIncrease this value to reduce sensitivity; decrease it to detect weaker vibration."),
    setting("vibration_sd_deadband", F32, "0.05", "Per-axis SD at or below this noise floor is treated and displayed as zero.\nSet to zero to disable; larger SD values are not reduced."),
    setting("recent_peak_windows", Usize, "32", "Number of motion windows retained by the rolling peak indicator."),
    // Orientation and the flip gesture.
    setting("calibration_duration_ms", U32, "3000", "Nominal IMU calibration sampling duration after the RGB test.\nDisplay transfers add a small amount of overhead."),
    setting("screen_vertical_cos_threshold", F32, "0.7", "Minimum normalized calibrated-axis component accepted as screen-up/down.\n0.7 accepts orientations within roughly 46° of directly face-up/down."),
    setting("orientation_filter_alpha", F32, "0.2", "Weight of each new 100 ms orientation reading in the low-pass filter."),
    setting("screen_down_hold_ms", U64, "500", "Continuous filtered face-down hold required to arm switching, in milliseconds."),
    setting("screen_release_debounce_windows", U8, "3", "Consecutive averaged 100 ms face-up windows required to complete a mode change."),
    // Shot timing.
    setting("start_confirm_seconds", U64, "2", "Delay before initial vibration is accepted as a shot."),
    setting("shot_timeout_seconds", U64, "99", "Maximum displayed shot duration before timeout."),
    setting("minimum_shot_seconds", U64, "5", "Completed shots shorter than this are discarded instead of retained."),
    setting("restart_confirm_seconds", U64, "3", "Continuous vibration required to replace a retained result with a new shot."),
    setting("completed_shot_hold_seconds", U64, "60", "Seconds to retain a completed shot result."),
    setting("progress_lap_seconds", U64, "25", "Seconds represented by one complete lap of the Timer-mode progress arc."),
    // Sleep.
    setting("sleep_enabled", Bool, "true", "Allow automatic display/IMU/MCU sleep. False keeps normal sampling and UI active."),
    setting("sleep_timeout_seconds", U64, "60", "Idle duration before the display backlight is turned off."),
    setting("sleep_wake_threshold_mg", U8, "50", "Hardware wake-on-motion acceleration change, in mg (not vibration SD)."),
    setting("sleep_check_interval_ms", U32, "500", "Low-power recovery/USB service interval; GPIO motion wakes immediately."),
    setting("allow_sleep_with_serial_connected", Bool, "true", "Allow sleep with a serial terminal connected, overriding USB sleep blocking.\nThe terminal does not itself wake the board. Independent of logging settings.\nDisable after testing to restore the normal USB/debugger sleep policy."),
    // Logging.
    setting("usb_logging_enabled", Bool, "true", "Emit serial diagnostics when a terminal is connected."),
    setting("usb_log_interval_ms", U64, "500", "Interval between periodic serial status lines."),
    setting("sleep_diagnostics_enabled", Bool, "true", "Log sleep decisions, wake sources, and failures. Does not change sleep policy."),
    // Battery.
    setting("use_battery", Bool, "true", "Enable battery monitoring, charge estimates, and battery UI.\nThe charger can power the measured rail over USB without a battery."),
    setting("battery_adc_reference_volts", F32, "3.3", "Battery ADC reference voltage for boards that report raw counts."),
    setting("battery_voltage_divider_ratio", F32, "0.5", "ADC-input/battery voltage divider ratio."),
    // Ring colors.
    setting("ring_first_lap_color", Rgb565, "[22, 28, 9]", "Ring palette in native RGB565 channel values: red 0..31, green 0..63,\nblue 0..31. Edge shades soften the outer pixel boundary of each arc."),
    setting("ring_first_lap_edge_color", Rgb565, "[11, 14, 5]", "First-lap arc edge shade."),
    setting("ring_second_lap_color", Rgb565, "[31, 20, 6]", "Second-lap arc color."),
    setting("ring_second_lap_edge_color", Rgb565, "[16, 10, 3]", "Second-lap arc edge shade."),
    setting("ring_track_color", Rgb565, "[0, 10, 8]", "Unfilled ring track color."),
    setting("ring_track_edge_color", Rgb565, "[0, 5, 4]", "Unfilled ring track edge shade."),
];

const BOARDS: &[&str] = &["esp32s3", "rp2040"];

fn main() {
    let workspace = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    println!("cargo:rerun-if-env-changed=SHOTTIMER_CONFIG");
    println!("cargo:rerun-if-changed=build.rs");

    let boards: Vec<_> = BOARDS
        .iter()
        .filter(|board| {
            env::var_os(format!("CARGO_FEATURE_BOARD_{}", board.to_uppercase())).is_some()
        })
        .collect();
    if boards.len() > 1 {
        panic!("only one board feature may be enabled, found {boards:?}");
    }
    let path = match env::var("SHOTTIMER_CONFIG") {
        Ok(path) => Some(workspace.join(path)),
        Err(_) => boards
            .first()
            .map(|board| workspace.join("configs").join(format!("{board}.toml"))),
    };

    let mut overrides = toml::Table::new();
    if let Some(path) = &path {
        println!("cargo:rerun-if-changed={}", path.display());
        let text = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("cannot read config {}: {error}", path.display()));
        overrides = text
            .parse()
            .unwrap_or_else(|error| panic!("invalid TOML in {}: {error}", path.display()));
        for key in overrides.keys() {
            if !SETTINGS.iter().any(|setting| setting.name == key) {
                panic!("{}: unknown setting `{key}`", path.display());
            }
        }
    }

    let source = path.as_ref().map_or_else(
        || "built-in defaults".into(),
        |path| path.display().to_string(),
    );
    let mut out = format!("// Generated by build.rs from {source}.\n");
    for setting in SETTINGS {
        let value = match overrides.get(setting.name) {
            Some(value) => value.clone(),
            None => toml::Value::from_str_default(setting.default),
        };
        let (ty, literal) = render(setting, &value)
            .unwrap_or_else(|error| panic!("{source}: `{}` {error}", setting.name));
        for line in setting.doc.lines() {
            writeln!(out, "/// {line}").unwrap();
        }
        writeln!(
            out,
            "pub const {}: {ty} = {literal};",
            setting.name.to_uppercase()
        )
        .unwrap();
    }
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out_dir.join("settings.rs"), out).unwrap();
}

trait FromDefault {
    fn from_str_default(text: &str) -> Self;
}

impl FromDefault for toml::Value {
    fn from_str_default(text: &str) -> Self {
        let table: toml::Table = format!("value = {text}").parse().unwrap();
        table["value"].clone()
    }
}

fn render(setting: &Setting, value: &toml::Value) -> Result<(&'static str, String), String> {
    let integer = |min: i64, max: i64| match value.as_integer() {
        Some(number) if (min..=max).contains(&number) => Ok(number.to_string()),
        Some(number) => Err(format!("must be in {min}..={max}, got {number}")),
        None => Err(format!("must be an integer, got {value}")),
    };
    Ok(match setting.kind {
        Bool => (
            "bool",
            value
                .as_bool()
                .ok_or_else(|| format!("must be true or false, got {value}"))?
                .to_string(),
        ),
        U8 => ("u8", integer(0, u8::MAX.into())?),
        U16 => ("u16", integer(0, u16::MAX.into())?),
        U32 => ("u32", integer(0, u32::MAX.into())?),
        U64 => ("u64", integer(0, i64::MAX)?),
        Usize => ("usize", integer(0, u32::MAX.into())?),
        F32 => {
            let number = value
                .as_float()
                .or_else(|| value.as_integer().map(|number| number as f64))
                .filter(|number| number.is_finite())
                .ok_or_else(|| format!("must be a finite number, got {value}"))?;
            ("f32", format!("{number:?}"))
        }
        Rgb565 => {
            let channels = value
                .as_array()
                .filter(|channels| channels.len() == 3)
                .ok_or_else(|| format!("must be [red, green, blue], got {value}"))?;
            let mut rendered = Vec::new();
            for (channel, max) in channels.iter().zip([31, 63, 31]) {
                match channel.as_integer() {
                    Some(number) if (0..=max).contains(&number) => {
                        rendered.push(number.to_string())
                    }
                    _ => {
                        return Err(format!(
                            "channels must be red 0..=31, green 0..=63, blue 0..=31, got {value}"
                        ));
                    }
                }
            }
            ("(u8, u8, u8)", format!("({})", rendered.join(", ")))
        }
        Axis => {
            let axis = value.as_str().unwrap_or_default();
            let vector = match axis {
                "+X" => "[1.0, 0.0, 0.0]",
                "-X" => "[-1.0, 0.0, 0.0]",
                "+Y" => "[0.0, 1.0, 0.0]",
                "-Y" => "[0.0, -1.0, 0.0]",
                "+Z" => "[0.0, 0.0, 1.0]",
                "-Z" => "[0.0, 0.0, -1.0]",
                _ => {
                    return Err(format!(
                        "must be one of \"+X\", \"-X\", \"+Y\", \"-Y\", \"+Z\", \"-Z\", got {value}"
                    ));
                }
            };
            ("[f32; 3]", vector.into())
        }
    })
}
