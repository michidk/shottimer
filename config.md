# Configuration

Settings are compile-time constants in [`crates/shottimer-core/src/settings.rs`](crates/shottimer-core/src/settings.rs).
Edit them, then [rebuild and flash the firmware](README.md#build-and-flash) to apply changes.

Common settings (see the source for the full list, including timing, orientation,
logging, battery calibration, and ring colors):

| Setting | Default | Meaning |
|---|---|---|
| `DEBUG_MODE_ENABLED` | `true` | Enable Debug mode and flip switching. |
| `START_IN_DEBUG_MODE` | `false` | Start in Debug mode when enabled. |
| `COLOR_TEST_ENABLED` | `true` | Run the RGB boot test before IMU calibration. |
| `SHOW_SHOT_HISTORY` | `true` | Show the three previous valid shot times. |
| `DISPLAY_ROTATION_DEGREES` | `270` | Clockwise rotation: `0`, `90`, `180`, or `270`; independent of flip detection. |
| `DISPLAY_BRIGHTNESS_PERCENT` | `100` | Backlight brightness, `0`–`100`. |
| `VIBRATION_SENSITIVITY_THRESHOLD` | `1.0` | Per-axis standard deviation threshold in m/s²; higher is less sensitive. |
| `SLEEP_ENABLED` | `true` | Enable automatic sleep. |
| `SLEEP_TIMEOUT_SECONDS` | `60` | Idle time before sleep; completed results use their own hold timeout. |
| `SLEEP_WAKE_THRESHOLD_MG` | `50` | Wake-on-motion threshold in mg; higher is less sensitive. |
| `ALLOW_SLEEP_WITH_SERIAL_CONNECTED` | `true` | Allow sleep with a serial terminal attached, overriding USB sleep blocking. |
| `USB_LOGGING_ENABLED` | `true` | Enable diagnostic logs. |
| `SLEEP_DIAGNOSTICS_ENABLED` | `true` | Log sleep decisions, wake sources, and failures. |
| `USE_BATTERY` | `true` | Enable battery monitoring and UI. |

For example, with `VIBRATION_SENSITIVITY_THRESHOLD = 1.0`, an SD of
`0.8 m/s²` is quiet, while `1.2 m/s²` counts as vibration. Exactly `1.0 m/s²`
does not trigger detection. Set the threshold above idle noise and below the
running machine's SD readings.

## Low-power sleep

Sleep turns off the backlight and LCD, switches the accelerometer to 128 Hz
wake-on-motion mode, and pauses rendering, battery sampling, and regular logs.
IMU INT2 wakes the MCU, with a status/recovery check every 500 ms. RP2040 uses
event-based CPU sleep with clocks and RAM retained; ESP32-S3 uses light sleep.

Movement wakes the display; normal vibration confirmation and flip detection
then resume. Battery readings never block sleep or wake the board. Disabling
`SLEEP_ENABLED` keeps normal sampling active; retained results still expire.

USB host detection normally prevents sleep. The default
`ALLOW_SLEEP_WITH_SERIAL_CONNECTED = true` permits RP2040 sleep with a serial
terminal attached; the terminal does not itself wake the board. Disable it to
restore the normal USB/debugger policy. Serial servicing and diagnostic logs
add overhead, so measure power savings without a terminal connected. Actual
current savings and wake sensitivity still require hardware testing.

## Battery and colors

With `USE_BATTERY = true`, Timer mode shows a green inner ring and charge
percentage when voltage rises or, on RP2040, a USB host is connected. Charge is
estimated from voltage; charging state and battery removal cannot be reliably
detected, and current draw is not measured.

RP2040 battery voltage is `ADC counts / 4095 × reference volts / divider ratio`.
ESP32-S3 uses calibrated ADC millivolts and its 200kΩ/100kΩ divider (ratio 1/3).

The `RING_*_COLOR` settings use RGB565 tuples: red/blue `0`–`31`, green `0`–`63`.
Invalid values fail at build time; edge colors soften arc boundaries.
`METERS_PER_SECOND_SQUARED_PER_COUNT` must match the configured ±8 g IMU range.

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
| LCD reset / backlight | 12 / 25 | 12 / 2 |
| Battery voltage ADC | 29 | 1 |
| Debug UART TX / RX | USB CDC | 43 / 44 (TX logging only) |
| Touch reset | — | 13 (touch not implemented) |
| LCD / IMU peripheral | SPI1 / I2C1 | SPI2 / I2C0 |

ESP32-S3 mapping and divider follow the
[Waveshare documentation](https://docs.waveshare.com/ESP32-S3-Touch-LCD-1.28)
and [schematic](https://files.waveshare.com/wiki/ESP32-S3-Touch-LCD-1.28/ESP32-S3-Touch-LCD-1.28-Sch.pdf).
