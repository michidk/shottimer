# shottimer

![Shot timer on an espresso machine](.github/images/shottimer.png)

Vibration-triggered espresso shot-timer firmware for Waveshare
**RP2040-LCD-1.28** and **ESP32-S3-Touch-LCD-1.28** boards.
Both use a 240×240 GC9A01 display and QMI8658 IMU.

## Features

- Automatic shot timing from machine vibration, with no button press required.
- Circular progress display and history of the three previous shots.
- Configurable vibration sensitivity, display, battery and sleep settings.
- Motion gesture to switch between the timer and diagnostic display.
- Shared Rust firmware for RP2040 and ESP32-S3 boards.
- Parametric, 3D-printable desktop and magnetic housings for the RP2040 board.

## Housing

Two printable, screw-free housings are available for the RP2040-LCD-1.28 board and
MakerFocus 2000 mAh battery. See the [housing project](housing/README.md) for
STLs, dimensions, assembly instructions and the parametric CAD build.
[Download latest housing artifact](https://nightly.link/michidk/shottimer/workflows/housing.yml/main/housing.zip)
(desktop stand and magnetic housing; two STLs and one STEP each, via nightly.link)

![Desktop stand and magnetic housing](.github/images/housing-overview.png)

## Workspace

- `crates/shottimer-core`: timing, motion/orientation, battery estimation, settings.
- `crates/shottimer-app`: shared firmware loop, UI, display and IMU drivers.
- `crates/firmware-rp2040`: RP2040 board backend.
- `crates/firmware-esp32s3`: ESP32-S3 board backend.

Both boards run the same application through embedded-hal drivers and a small
`Platform` interface that separates shared behavior from board-specific hardware.
Dependencies flow from firmware → app → core; firmware can also use core directly.
Shared crates must not depend on MCU HALs.

ESP32-S3 builds successfully but has not been hardware-tested; touch, Wi-Fi, BLE
and PSRAM are not used yet.

## Behavior

After the three-second RGB display test, keep the display still and face-up during
boot calibration. The default configuration then starts in Timer mode.

- Detects vibration from the largest per-axis standard deviation across ten
  acceleration samples taken at 10 ms intervals.
- Starts a shot after two seconds of continued vibration and counts in calibrated
  975 ms increments, resetting at 99 displayed seconds.
- Stops after a quiet timer bucket and corrects the result to the last detected
  vibration, excluding the stop-confirmation delay. Shots shorter than five
  seconds are discarded.
- Retains valid results for 60 seconds. Three seconds of continued vibration
  replaces a retained result; the new timer first appears at `3`.
- Shows three previous valid shot times, newest first, excluding the current
  result. History survives resets, mode switches, and sleep, but clears on reboot.

The progress arc fills in two brown shades over two 25-second laps, remaining
full after 50 seconds while the number continues.

Hold the LCD face-down for 0.5 seconds, then turn it face-up to toggle Timer and
Debug modes. Orientation filtering adds some response time; switching resets
the active timer. Set `DEBUG_MODE_ENABLED = false` to disable Debug mode and
this gesture.

## Debug mode

Debug mode shows IMU address/errors, acceleration mean and standard deviation,
filtered orientation, vibration threshold and rolling peak, timer/idle state,
and battery voltage, estimated charge, ADC value, and trend.

Both boards log acceleration and orientation twice per second: RP2040 through
USB CDC, ESP32-S3 through its CH343 UART bridge at 115200 baud. Logging and its
interval are configurable; disabling logs leaves the serial interface available.

## Configuration

See [config.md](config.md) for compile-time settings, vibration tuning, sleep,
battery and display options, and hardware assignments. Changes require rebuilding
and flashing the firmware.

## Build and flash

Install [rustup](https://rustup.rs) and use its Cargo/Rust binaries (put
`~/.cargo/bin` before any system Rust in `PATH`). Rustup installs the pinned
toolchain, components and RP2040 target from `rust-toolchain.toml`. Host checks
require no board:

```sh
cargo fmt --all -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

### RP2040

```sh
rustup target add thumbv6m-none-eabi
cargo install elf2uf2-rs
cargo rp2040
```

Hold **BOOT** while connecting USB, then flash:

```sh
cargo run --release -p firmware-rp2040 --target thumbv6m-none-eabi
```

ELF: `target/thumbv6m-none-eabi/release/shottimer-rp2040`.
The target runner converts it to UF2 and copies it to the BOOTSEL drive.

### ESP32-S3

Install the [Espressif Rust toolchain](https://docs.espressif.com/projects/rust/book/getting-started/toolchain.html):

```sh
cargo install espup espflash
espup install --targets esp32s3
. ~/export-esp.sh
cargo +esp esp32s3
```

Source the export script in each shell; if a custom export path was used, source
that instead. The `esp` toolchain and `-Zbuild-std=core` are required for Xtensa.

Connect the board over USB and flash via its CH343 serial bridge:

```sh
cargo +esp run --release -p firmware-esp32s3 \
  --target xtensa-esp32s3-none-elf -Zbuild-std=core
```

ELF: `target/xtensa-esp32s3-none-elf/release/shottimer-esp32s3`.
The runner uses `espflash` with ESP32-S3, 16 MB flash, and serial monitoring.
If automatic download fails, hold **BOOT**, press **RESET**, and retry.
Do not flash an RP2040 UF2 onto ESP32-S3.

## Acknowledgements and licensing

Shot detection and the circular timer concept were inspired by
[`lspr98/profitec-go-waterlevel-shottimer`](https://github.com/lspr98/profitec-go-waterlevel-shottimer).
This is an independent Rust implementation and contains none of that project's
source code. The referenced repository did not declare a software license when
this acknowledgement was written.

This project is available under either the [Apache License 2.0](LICENSE-APACHE)
or [MIT License](LICENSE-MIT).
