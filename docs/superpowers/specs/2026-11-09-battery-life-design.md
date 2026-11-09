# Battery life on both boards

Status: approved by the user on 2026-11-09.

## Goal

Reduce battery consumption on both the Waveshare RP2040-LCD-1.28 and
ESP32-S3-Touch-LCD-1.28 while keeping automatic shot detection, touch/flip
controls, and shot history working. No battery-life multiplier is claimed
without measurements on the boards.

## Approaches

The recommended first pass combines more economical defaults with less
frequent battery measurement in the shared application. Both boards benefit,
and the existing host tests can exercise the shared behavior.

Changing only configuration is smaller but leaves unnecessary ADC work in the
application. Deeper MCU sleep could provide larger standby savings, especially
on RP2040, but changes clock and wake behavior and needs physical validation.
It is a separate follow-up, not a prerequisite for this first pass.

## Proposed changes

- Set default backlight brightness to 50% on both boards. It remains
  configurable; readability must be checked on the physical displays.
- Set the idle sleep timeout to 60 seconds on both boards. Keep completed
  results visible for their existing 60 seconds.
- Disable regular serial logging and sleep diagnostics by default. Keep
  Debug mode and the settings that re-enable logging.
- Add a configurable battery sampling interval, defaulting to 1,000 ms.
  Read immediately when no sample exists, cache the last successful reading,
  and limit failed attempts to the same cadence. Refresh on resume from sleep.
  Preserve the voltage trend's elapsed-time calculations and verify the
  slower filter response does not produce incorrect charge indications.
- Keep built-in defaults, both board configurations, and documentation in
  agreement.

These are implementation recommendations, not user-selected numeric settings.

## Behavior to preserve

Keep the accelerometer's active and wake-on-motion configuration, 10 ms shot
sampling delay, vibration thresholds, shot timing, display result retention,
USB sleep policy, and 500 ms sleep recovery checks unchanged. The current wake
handling deliberately distinguishes the GPIO level from confirmed motion;
extending the recovery interval before verifying hardware wake behavior could
delay detection when a GPIO edge is missed.

## Verification

Exercise battery sampling cadence, startup, read failures, recovery, and wake
refresh through the shared application tests. Run the existing shot timing,
sleep/wake, battery trend, and display tests with both board configurations.
Run formatting, host linting, and both firmware builds when their toolchains
are available. Report any unavailable cross-compilation checks explicitly.

Hardware acceptance requires readable displays, reliable first-shot detection
after idle sleep, retained history across wake, and battery-side current
measurements while active and asleep. Compare runtime only with the same
battery and usage pattern. Firmware tests cannot establish the current saved.
