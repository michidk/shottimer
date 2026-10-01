# Multi-board architecture

## 1. Starting point

The original firmware loop combined RP2040 setup, ADC/USB/power management,
display rendering, sampling, calibration, and shot detection in one entrypoint.
Porting that entrypoint directly would duplicate behavior and board assumptions.

## 2. Fast feedback

Run `bash scripts/verify.sh` for formatting, host tests, boundary checks, Clippy,
and the RP2040 release build. Source the espup environment and add `--esp`
to check and build ESP32-S3 too. Host tests require no connected hardware.

## 3. Module map

| Crate | Owns | Does not own |
|---|---|---|
| shottimer-core | Shot state/history, vibration statistics, orientation, battery trends, settings | Peripheral drivers, rendering |
| shottimer-app | Sampling/calibration loop, both screens, shared GC9A01/QMI8658 drivers | MCU setup, ADC scaling, USB/UART, CPU sleep |
| firmware-rp2040 | RP2040 pins, ADC, PWM, USB CDC, event wake | Shot detection or UI logic |
| firmware-esp32s3 | ESP32-S3 pins, calibrated ADC, LEDC, UART, light sleep | Shot detection or UI logic |

## 4. Deep modules

Both firmware binaries call the same application. They implement a small board
boundary instead of selecting chip features throughout the application.
Rendering and transport internals remain private to the application crate.
MCU-independent libraries provide display/sensor protocols; HALs provide buses.

## 5. Interface contracts

- Core: `ShotTimer::update_with_sleep_policy(now_ms, vibrating, allow_sleep)`
  returns a shot state. Valid shots enter history; short shots do not.
  `FlipModeSwitch::update(mean)` emits a mode only after a held-down/up gesture.
  `BatteryMonitor::update_voltage(raw_counts, volts, now_ms)` keeps ADC scaling
  outside the trend/charge model.
- Application: `run<P: Platform>(board: P) -> !` owns the firmware loop.
  `Platform` supplies monotonic milliseconds, signed ±8 g IMU counts, physical
  battery volts, display writes, logging, and reversible low-power operations.
  A region refresh uses a complete 240×240 RGB565 framebuffer.
- Drivers: `Imu<I2c>` and `Display<SpiBus, OutputPin, ...>` own device protocols,
  initialization, and sleep/wake commands through embedded-hal traits.
- Board backends: setup errors stop startup; communication failures return
  `HardwareError`. An IMU sleep/read failure causes recovery, not a fabricated
  quiet sample. Sleep preserves RAM/history and periodically rechecks voltage.

For example, a normal sample window feeds shared vibration detection on either
board. An IMU read error skips that window and increments the same debug count.
UART logging cannot claim a host or debugger is attached without a sense signal.

## 6. Filesystem migration

Pure modules moved from `src/` to `crates/shottimer-core/src/`.
UI and display transport moved into `crates/shottimer-app/src/`.
The firmware loop now lives in its private `engine` module.
RP2040 hardware definitions, linker memory layout, and build script moved to
`crates/firmware-rp2040/`; ESP32-S3 has a separate entrypoint/hardware mapping.
The workspace defaults to the two shared crates for host checks. Firmware
selection always names a package and CPU target; no compatibility shim remains.

## 7. Boundary enforcement

Dependency direction is firmware → app → core; firmware may also use core.
Rust private modules hide screen/engine/transport implementation details.
The verification script rejects board HALs/firmware in the app dependency tree.
Do not add chip feature flags, unsafe register access, or board pin numbers to
the shared application. Put new boards in another firmware crate.

## 8. Testing

Core contracts cover timing, corrected stop values, discarded runs, history,
retention, sleep policy, calibrated orientation, and battery trends.
Application tests cover complete refresh regions, old-pixel removal, history,
charge arcs, display transport, and a fake-board run through boot/calibration,
sampling, logging, sleep and wake. A separate ADC test ensures a board voltage
is not accidentally interpreted using the RP2040 divider.
On-device acceptance still needs RGB/calibration, vibration, flip, battery,
UART/USB logging, and wake tests. ESP32 touch, Wi-Fi, BLE and PSRAM are unused.

## 9. Checkpoints

1. Move pure modules; retain and run their contract tests.
2. Introduce Platform and move shared rendering/drivers/loop; build RP2040.
3. Add ESP32-S3 pin/ADC/logging/power backend; build the Xtensa target.
4. Run host and both target checks; validate on hardware before flashing changes
   to deployed devices. Git can revert each checkpoint without changing shared
   APIs or requiring an on-device data migration.
