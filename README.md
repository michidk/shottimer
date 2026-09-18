# shottimer

Rust shot-timer and diagnostic firmware for the **Waveshare RP2040-LCD-1.28**
(240×240 GC9A01 round LCD + QMI8658 IMU). It intentionally contains no
water-level support.

The firmware uses the board definition and display example from the upstream
[`rp-rs/rp-hal-boards`](https://github.com/rp-rs/rp-hal-boards) Waveshare
RP2040-LCD-1.28 BSP. The screen is
driven by `gc9a01a_driver` through a black-backed framebuffer. Only changed
screen regions are transmitted during normal operation, avoiding both stale
pixels and full-screen tearing. The QMI8658 is handled by `ph-qmi8658`; the
timer state machine and vibration statistics remain application code. Their
behavior and the timer-screen concept were inspired by
[`lspr98/profitec-go-waterlevel-shottimer`](https://github.com/lspr98/profitec-go-waterlevel-shottimer).

## Shot detection

The detection behavior follows that project's documented behavior: ten accelerometer samples
are collected 10 ms apart, vibration is the largest per-axis standard deviation,
and a possible shot must still be vibrating after a two-second confirmation
delay. Once started, the timer remains active while vibration is observed within
each aligned 975 ms timer bucket. One positive vibration window is sufficient
to keep a bucket active; a bucket with no positive window ends the shot.
Displayed whole seconds use the same 975 ms calibrated increment. An active shot
resets at the configurable limit of 99 displayed seconds by default. The
completed shot time remains visible for 60 seconds or until motion starts a new
shot candidate, whichever happens first.

The default UI follows the same concept with a large centered timer, an outer
progress arc. Its configurable lap length defaults to 25 seconds: one brown
shade fills from 0–25 seconds, then a second brown shade overlays it from 25–50
seconds. The arc remains full after 50 seconds while the numeric timer continues.
It shows `0` while ready and elapsed seconds without a unit suffix while timing; battery
information appears only in Debug mode. Holding the device LCD-face-down selects
Debug mode. Returning it to any non-down orientation selects Timer mode and
allows a later face-down transition to trigger again. The orientation signal is
low-pass filtered, and screen-down must remain continuously detected for at
least one second before Debug mode activates. The down cone is approximately
60° from directly face-down. Returning to Timer mode uses a shorter debounce.
Entering Debug mode resets an active timer.
Returning from Debug mode also resets all shot-timer state and starts Timer mode
at `0`.

## What Debug mode shows

- detected QMI8658 I²C address;
- mean X/Y/Z acceleration in m/s²;
- the current screen direction (`UP`, `DOWN`, `SIDE`, or `TILTED`);
- standard deviation for each axis over a 100 ms window;
- the live and rolling 32-frame peak deviations and configured vibration and
  UI-toggle limits;
- battery connected status, voltage, and approximate single-cell LiPo charge
percentage from the board's GP29 ADC divider;
- an explicit error screen if the IMU is absent or stops responding.

The board has no current-sense circuit, so firmware cannot measure its amp
draw. That requires an external inline current monitor such as an INA219 or
INA226. The onboard charger's 1 A rating is a limit, not a current measurement.
The displayed charge percentage is a voltage-curve estimate and is less
accurate while charging or under load.

The sampling settings mirror the referenced project: 10 samples, 10 ms between
samples, ±8 g accelerometer range, and a 1000 Hz sensor output rate. The tuned
vibration threshold is 10 m/s². Orientation switching uses the mean Z axis with
hysteresis and a screen-relative sign verified from the live sensor readings.
Change the calibration constants in `src/settings.rs` after recording idle,
handling, and actual-shot values.

At boot, the display runs a basic pixel test: solid red, green, and blue for one
second each, followed by Timer mode.

## Hardware mapping

| Function | RP2040 GPIO |
|---|---:|
| IMU SDA / SCL | 6 / 7 |
| LCD DC / CS | 8 / 9 |
| LCD SCK / MOSI | 10 / 11 |
| LCD reset / backlight | 12 / 25 |
| Battery voltage ADC | 29 |

These are the board's built-in connections; no external sensor is required.

## Build and flash

Install Rust, then:

```sh
rustup target add thumbv6m-none-eabi
cargo install elf2uf2-rs
cargo build --release
```

On WSL, if `type -a rustc` lists `/usr/bin/rustc` before the rustup copy, put
rustup first for the current shell before building:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

Hold **BOOT**, connect the board over USB, and run:

```sh
cargo run --release
```

`cargo run` uses `elf2uf2-rs -d` to find the RP2040 USB boot drive and copy the
firmware. Do not power the board simultaneously from unsafe or unrelated power
supplies.

## Calibration run

Record the displayed `peak sd` in four situations: untouched device, ordinary
handling, the vibration produced during an actual shot, and other nearby
vibration. A useful threshold is above the worst non-shot value and comfortably
below the lowest shot value. If those ranges overlap, adjust the mounting or
sample window before relying on automatic timing.

## Acknowledgements and licensing

The shot-detection behavior, timing defaults, and circular timer-screen concept
were inspired by
[`lspr98/profitec-go-waterlevel-shottimer`](https://github.com/lspr98/profitec-go-waterlevel-shottimer).
This is an independent Rust implementation and does not include that project's
source code. The upstream repository did not declare a software license when
this acknowledgement was written; this project's license does not grant rights
to the upstream project.

The shottimer source is available under your choice of the
[Apache License 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT). Third-party
components and embedded fonts retain their own licenses; see
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
