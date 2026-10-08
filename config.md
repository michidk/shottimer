# Configuration

Settings are compiled into the firmware from a TOML config set per board:

| Board | Config set |
|---|---|
| ESP32-S3-Touch-LCD-1.28 | [`configs/esp32s3.toml`](configs/esp32s3.toml) |
| RP2040-LCD-1.28 | [`configs/rp2040.toml`](configs/rp2040.toml) |

Each firmware build selects its board's file automatically. Edit it, then
[rebuild and flash the firmware](README.md#build-and-flash) to apply changes. To
use a private file instead, set `SHOTTIMER_CONFIG` to its path (relative to the
repository root), e.g. `SHOTTIMER_CONFIG=configs/my-machine.toml cargo +esp esp32s3`.

Both files list every setting with comments. Keys left out use the built-in
default from [`crates/shottimer-core/build.rs`](crates/shottimer-core/build.rs),
which also documents each setting. Unknown keys and out-of-range values fail the
build with the file and key named. In code, each key becomes an upper-case
constant in `shottimer_core::settings`, e.g. `display_rotation_degrees` →
`DISPLAY_ROTATION_DEGREES`.

Common settings:

| Setting | ESP32-S3 | RP2040 | Meaning |
|---|---|---|---|
| `debug_mode_enabled` | `true` | `true` | Enable Debug mode and its switching gesture. |
| `start_in_debug_mode` | `false` | `false` | Start in Debug mode when enabled. |
| `color_test_enabled` | `true` | `true` | Run the RGB boot test. |
| `touch_mode_enabled` | `true` | `false` | Swipe to switch modes and skip calibration; see below. |
| `swipe_min_distance_px` | `60` | `60` | Minimum finger travel recognized as a swipe. |
| `screen_up_axis` | `"-Z"` | unused | Accelerometer axis out of the screen, replacing calibration in touch mode. |
| `show_shot_history` | `true` | `true` | Show the three previous valid shot times. |
| `display_rotation_degrees` | `270` | `270` | Clockwise rotation: `0`, `90`, `180`, or `270`; independent of flip detection. |
| `display_brightness_percent` | `100` | `100` | Backlight PWM brightness, `0`–`100`. |
| `vibration_sensitivity_threshold` | `1.0` | `1.0` | Per-axis standard deviation threshold in m/s²; higher is less sensitive. |
| `sleep_enabled` | `true` | `true` | Enable automatic sleep. |
| `sleep_timeout_seconds` | `60` | `60` | Idle time before sleep; completed results use their own hold timeout. |
| `sleep_wake_threshold_mg` | `50` | `50` | Wake-on-motion threshold in mg; higher is less sensitive. |
| `allow_sleep_with_serial_connected` | `true` | `true` | Allow sleep with a serial terminal attached, overriding USB sleep blocking. |
| `usb_logging_enabled` | `true` | `true` | Enable diagnostic logs. |
| `sleep_diagnostics_enabled` | `true` | `true` | Log sleep decisions, wake sources, and failures. |
| `use_battery` | `true` | `true` | Enable battery monitoring and UI. |
| `battery_voltage_divider_ratio` | `0.333333` | `0.5` | ADC-input/battery voltage ratio. |

## Touch mode

With `touch_mode_enabled = true` on a board with a touch controller, a vertical
swipe toggles Timer and Debug modes, and the face-down flip gesture is disabled.
Directions follow `display_rotation_degrees`, so "up" is toward the top of the
UI. Horizontal swipes are recognized and logged but reserved for future use.
Boot calibration is skipped; `screen_up_axis` defines face-up for the Debug-mode
orientation readout. If the controller does not respond at boot, the firmware
logs it and falls back to calibration and flip switching. Swipes do not wake the
board from sleep.

## Vibration threshold

For example, with `vibration_sensitivity_threshold = 1.0`, an SD of
`0.8 m/s²` is quiet, while `1.2 m/s²` counts as vibration. Exactly `1.0 m/s²`
does not trigger detection. Set the threshold above idle noise and below the
running machine's SD readings.

## Low-power sleep

Sleep turns off the backlight and LCD, switches the accelerometer to 128 Hz
wake-on-motion mode, and pauses rendering, battery sampling, and regular logs.
IMU INT2 wakes the MCU, with a status/recovery check every 500 ms. RP2040 uses
event-based CPU sleep with clocks and RAM retained; ESP32-S3 uses light sleep.

Movement wakes the display; normal vibration confirmation and mode switching
then resume. Battery readings never block sleep or wake the board. Disabling
`sleep_enabled` keeps normal sampling active; retained results still expire.

USB host detection normally prevents sleep. The default
`allow_sleep_with_serial_connected = true` permits RP2040 sleep with a serial
terminal attached; the terminal does not itself wake the board. Disable it to
restore the normal USB/debugger policy. Serial servicing and diagnostic logs
add overhead, so measure power savings without a terminal connected. Actual
current savings and wake sensitivity still require hardware testing.

## Battery and colors

With `use_battery = true`, Timer mode shows a green inner ring and charge
percentage when voltage rises or, on RP2040, a USB host is connected. Charge is
estimated from voltage; charging state and battery removal cannot be reliably
detected, and current draw is not measured.

RP2040 battery voltage is `ADC counts / 4095 × reference volts / divider ratio`.
ESP32-S3 uses calibrated ADC millivolts and its 200kΩ/100kΩ divider (ratio 1/3).
Both divider ratios come from `battery_voltage_divider_ratio`.

The `ring_*_color` settings use `[red, green, blue]` RGB565 channels: red/blue `0`–`31`, green `0`–`63`.
Invalid values fail at build time; edge colors soften arc boundaries.
`METERS_PER_SECOND_SQUARED_PER_COUNT` in `settings.rs` is fixed in code and must
match the configured ±8 g IMU range.

## Hardware assignments

RP2040 GPIO and SPI/I2C peripheral assignments are configured in
[`crates/firmware-rp2040/src/hardware.rs`](crates/firmware-rp2040/src/hardware.rs). Edit the `gpN` fields and peripheral
selections, then rebuild. The HAL checks pin reuse and supported SPI/I2C/ADC
assignments at compile time. Changing firmware assignments requires matching
physical wiring; the onboard display and IMU connections are fixed.
The backlight also needs the matching PWM slice and channel in
`select_backlight_pwm!`: GP25 uses slice `pwm4`, `channel_b`. The HAL verifies
that the chosen PWM channel supports the backlight pin.

ESP32-S3 assignments are in
[`crates/firmware-esp32s3/src/hardware.rs`](crates/firmware-esp32s3/src/hardware.rs).
Its LEDC timer/channel are set up in the board entrypoint.

### Default hardware mapping

Assignments match onboard wiring and require no external vibration sensor.
Changing them requires matching physical wiring.

| Function | RP2040 GPIO | ESP32-S3 GPIO |
|---|---:|---:|
| IMU SDA / SCL | 6 / 7 | 6 / 7 |
| IMU INT2 (sleep wake) | 24 | 3 |
| LCD DC / CS | 8 / 9 | 8 / 9 |
| LCD SCK / MOSI | 10 / 11 | 10 / 11 |
| LCD reset / backlight | 12 / 25 | 14 / 2 |
| Battery voltage ADC | 29 | 1 |
| Debug UART TX / RX | USB CDC | 43 / 44 (TX logging only) |
| Touch SDA / SCL (shared with IMU) | — | 6 / 7 |
| Touch reset / INT | — | 13 / 5 |
| LCD / IMU peripheral | SPI1 / I2C1 | SPI2 / I2C0 |

ESP32-S3 mapping and divider follow the
[Waveshare documentation](https://docs.waveshare.com/ESP32-S3-Touch-LCD-1.28)
and [schematic](https://files.waveshare.com/wiki/ESP32-S3-Touch-LCD-1.28/ESP32-S3-Touch-LCD-1.28-Sch.pdf).
