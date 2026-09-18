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
- Retains the completed value for 60 seconds or until a new shot begins.
- Resets an active shot at 99 displayed seconds.

Timer mode shows the elapsed value and a two-lap progress arc. The first brown
shade fills over 25 seconds; the second shade overlays it during the next 25
seconds. The arc remains full after 50 seconds while the number continues.

Holding the LCD face-down for at least one second enters Debug mode. The
orientation signal is low-pass filtered and accepts a roughly 60° face-down
cone. Returning to any non-down orientation returns to Timer mode. Both mode
changes reset the shot timer.

At boot, the display shows solid red, green, and blue for one second each.

## Debug mode

Debug mode shows:

- QMI8658 address;
- mean and standard deviation for X/Y/Z acceleration;
- `UP`, `DOWN`, `SIDE`, or `TILTED` screen direction;
- live and rolling peak vibration;
- timer state;
- LiPo connection, voltage, and estimated charge percentage;
- IMU initialization or read errors.

Charge percentage is estimated from cell voltage and is less accurate while
charging or under load. The board has no current-sense circuit, so it cannot
measure current draw without external hardware.

## Configuration

Calibration values are in [`src/settings.rs`](src/settings.rs), including:

- vibration threshold;
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
