# Battery life implementation plan

> Implement in this session using superpowers:executing-plans.

**Goal:** Reduce consumption on both boards using the approved shared changes.
**Architecture:** Keep board hardware interfaces intact. Schedule ADC attempts in the shared application and expose the interval through existing generated settings.
**Tech stack:** Rust, embedded-hal, TOML, Cargo.
**Spec:** ../specs/2026-11-09-battery-life-design.md

## Constraints

Use 50% brightness, 60-second idle sleep, disabled logs, and a 1,000 ms battery interval. Preserve shot timing and completed-result retention. Use the configured shared Cargo target directory.

## Review focus

Startup without a battery sample; repeated ADC failures; first sample after wake; disabled battery monitoring; voltage trends at the slower cadence.

## Tasks

- [x] Add battery_sample_interval_ms to build.rs, both configs, and settings validation. Update brightness, sleep, and logging defaults plus documentation.
- [x] Add behavioral tests for cadence, failure throttling, recovery, and resume refresh in engine.rs; observe failure before implementing scheduling.
- [x] Track the last ADC attempt, reuse successful status, throttle failures, and invalidate the attempt deadline on resume.
- [x] Run formatting, host tests and lint, both board configuration test runs, and available firmware builds. Check existing trend tests and add a cadence scenario if needed.
- [x] Review the diff, publish a signed feature-branch commit through GitHub API, verify its signature and co-authors, and provide the compare link.

## Verification record

The new ADC cadence test failed before scheduling was implemented and passed afterward. Default host suite: 83 passed. RP2040 configuration suite: 83 passed. RP2040 release build passed. A fresh review found no actionable defects. ESP32-S3 cross-build cannot run locally because the esp toolchain is absent; shared code is exercised with its TOML configuration.

The ESP32-S3 configuration exposed a pre-existing test with hardcoded RP2040 ADC counts. The test now calculates the counts for a 4.1 V battery using the configured divider and reference.
