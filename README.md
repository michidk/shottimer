# shottimer

Vibration-triggered espresso shot-timer firmware for the
**Waveshare RP2040-LCD-1.28** (RP2040, 240×240 GC9A01 display, and QMI8658 IMU).
There is no water-level or pump integration.

## Behavior

- Samples acceleration ten times at 10 ms intervals.
- Detects vibration from the largest per-axis standard deviation.
- Confirms a possible shot after two seconds of continued vibration.
- Updates elapsed time in calibrated 975 ms increments.
- Ends a shot after a timer bucket contains no vibration.
- Discards completed shots shorter than five seconds without showing a result.
- Retains the completed value for 60 seconds. New vibration must continue for
  three seconds before replacing it; the new timer then first appears at `3`.
- Resets an active shot at 99 displayed seconds.

Timer mode shows the elapsed value and a two-lap progress arc. The first brown
shade fills over 25 seconds; the second shade overlays it during the next 25
seconds. The arc remains full after 50 seconds while the number continues.

Holding the LCD face-down for at least one second arms a mode change. Turning it
back face-up then toggles between Timer and Debug modes. Orientation uses
averaged 100 ms sample windows and accepts a roughly 46° up/down cone. A mode
change resets the shot timer.

At boot, the display first shows solid red, green, and blue for one second each.
After the color test, a dedicated IMU calibration screen shows live averaged
X/Y/Z acceleration and sample progress. Keep the display still and face-up
during calibration. The strongest gravity axis and its sign become the
screen-up reference; the other two axes are discarded so a small boot-time tilt
cannot redefine the screen plane.

The default configuration then starts in Timer mode. Hold the display face-down
for one second and turn it back up to enter Debug mode.

## Debug mode

Debug mode shows:

- QMI8658 address and cumulative IMU read-error count since boot;
- mean and standard deviation for X/Y/Z acceleration;
- filtered `UP`, `DOWN`, `SIDE`, or `TILTED` screen direction and its numeric
  score (`+1.00` is face-up and `-1.00` is face-down);
- vibration detected (`YES`/`NO`), live deviation, threshold, and rolling peak;
- timer state and remaining confirmation, result-hold, or timeout duration;
- LiPo connection, voltage, estimated charge percentage, raw ADC value, and
  filtered voltage trend over time;
- IMU initialization or read errors.

The firmware also exposes a USB CDC debug serial port. It emits X/Y/Z
acceleration, the calibrated screen-normal axis, raw and filtered orientation
scores, and the detected direction twice per second.

Charge percentage is estimated from cell voltage and is less accurate while
charging or under load. The board does not expose the charger's status output
or contain a current-sense circuit, so firmware cannot reliably report charging
state or current draw without additional hardware.

The debug display therefore labels the measured trend as `VOLTAGE RISING`,
`VOLTAGE STABLE`, or `VOLTAGE FALLING`; it does not claim this is the charger's
authoritative state.

## Configuration

Calibration values are in [`src/settings.rs`](src/settings.rs), including:

- vibration sensitivity threshold (higher is less sensitive);
- initial Debug/Timer mode;
- minimum retained-shot duration;
- retained-result restart confirmation duration;
- shot timeout;
- completed-result hold duration;
- progress-lap duration;
- orientation angle, filtering, and debounce.

## Build and flash

```sh
rustup target add thumbv6m-none-eabi
cargo install elf2uf2-rs
cargo build --release
```

Hold **BOOT**, connect the board over USB, then run:

```sh
cargo run --release
```

## Hardware mapping

| Function | RP2040 GPIO |
|---|---:|
| IMU SDA / SCL | 6 / 7 |
| LCD DC / CS | 8 / 9 |
| LCD SCK / MOSI | 10 / 11 |
| LCD reset / backlight | 12 / 25 |
| Battery voltage ADC | 29 |

These are built-in board connections; no external vibration sensor is needed.

## Acknowledgements and licensing

Shot detection and the circular timer concept were inspired by
[`lspr98/profitec-go-waterlevel-shottimer`](https://github.com/lspr98/profitec-go-waterlevel-shottimer).
This is an independent Rust implementation and contains none of that project's
source code. The referenced repository did not declare a software license when
this acknowledgement was written.

This project is available under either the [Apache License 2.0](LICENSE-APACHE)
or [MIT License](LICENSE-MIT).
