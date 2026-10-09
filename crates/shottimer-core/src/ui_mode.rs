//! Orientation-controlled switching between Timer and Debug modes.

use crate::settings::{
    DEBUG_MODE_ENABLED, ORIENTATION_FILTER_ALPHA, SCREEN_DOWN_HOLD_MS,
    SCREEN_RELEASE_DEBOUNCE_WINDOWS, SCREEN_VERTICAL_COS_THRESHOLD,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiMode {
    Timer,
    Debug,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenDirection {
    Down,
    Up,
    Side,
    Tilted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenAxis {
    X,
    Y,
    Z,
}

impl ScreenAxis {
    const ALL: [Self; 3] = [Self::X, Self::Y, Self::Z];

    const fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }

    pub const fn label(self) -> char {
        match self {
            Self::X => 'X',
            Self::Y => 'Y',
            Self::Z => 'Z',
        }
    }
}

/// Standard gravity in m/s², the expected magnitude of a still calibration.
pub const STANDARD_GRAVITY: f32 = 9.806_65;
/// Accepted relative deviation of the calibration mean from one gravity.
const CALIBRATION_GRAVITY_TOLERANCE: f32 = 0.2;

/// Why a boot calibration window cannot define the screen-up axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CalibrationError {
    /// Too many IMU reads failed during the window.
    TooFewSamples { samples: u32, required: u32 },
    /// The mean acceleration (m/s²) is not close to one gravity.
    NotGravity { magnitude: f32 },
}

/// Accumulates screen-up acceleration samples, in m/s².
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OrientationCalibration {
    total: [f32; 3],
    samples: u32,
}

impl OrientationCalibration {
    pub const fn new() -> Self {
        Self {
            total: [0.0; 3],
            samples: 0,
        }
    }

    pub fn add(&mut self, sample: [f32; 3]) {
        for (total, value) in self.total.iter_mut().zip(sample) {
            *total += value;
        }
        self.samples += 1;
    }

    pub const fn sample_count(self) -> u32 {
        self.samples
    }

    pub fn mean(self) -> Option<[f32; 3]> {
        if self.samples == 0 {
            return None;
        }
        let divisor = self.samples as f32;
        Some(self.total.map(|total| total / divisor))
    }

    /// Returns the mean only if at least `required` reads succeeded and it
    /// looks like gravity, so a failed or disturbed window is never trusted.
    pub fn validate(self, required: u32) -> Result<[f32; 3], CalibrationError> {
        let mean = self.mean().filter(|_| self.samples >= required).ok_or(
            CalibrationError::TooFewSamples {
                samples: self.samples,
                required,
            },
        )?;
        let magnitude = libm::sqrtf(mean.iter().map(|value| value * value).sum());
        // A NaN magnitude would pass the tolerance comparison below.
        if !magnitude.is_finite()
            || (magnitude - STANDARD_GRAVITY).abs()
                > STANDARD_GRAVITY * CALIBRATION_GRAVITY_TOLERANCE
        {
            return Err(CalibrationError::NotGravity { magnitude });
        }
        Ok(mean)
    }
}

pub struct FlipModeSwitch {
    mode: UiMode,
    filtered_vertical: Option<f32>,
    debug_enabled: bool,
    /// False when another input, such as touch, switches modes.
    flip_enabled: bool,
    down_since_ms: Option<u64>,
    last_update_ms: u64,
    release_windows: u8,
    /// A completed hold has toggled the mode and is waiting for release.
    flip_armed: bool,
    screen_normal_axis: ScreenAxis,
    screen_up_sign: f32,
}

impl FlipModeSwitch {
    pub const fn new(mode: UiMode) -> Self {
        Self::with_debug_enabled(mode, DEBUG_MODE_ENABLED)
    }

    pub const fn with_debug_enabled(mode: UiMode, debug_enabled: bool) -> Self {
        Self {
            mode: if debug_enabled { mode } else { UiMode::Timer },
            debug_enabled,
            flip_enabled: true,
            filtered_vertical: None,
            down_since_ms: None,
            last_update_ms: 0,
            release_windows: 0,
            flip_armed: false,
            screen_normal_axis: ScreenAxis::Z,
            screen_up_sign: 1.0,
        }
    }

    /// Determines which accelerometer axis and sign mean display face-up.
    ///
    /// Only the strongest axis and its sign are retained. Using the complete
    /// measured gravity vector would incorrectly make any boot-time tilt the
    /// permanent screen normal.
    pub fn calibrated(mode: UiMode, screen_up_mean: [f32; 3]) -> Self {
        let mut switch = Self::new(mode);
        switch.screen_normal_axis = ScreenAxis::X;
        for axis in ScreenAxis::ALL.into_iter().skip(1) {
            if screen_up_mean[axis.index()].abs()
                > screen_up_mean[switch.screen_normal_axis.index()].abs()
            {
                switch.screen_normal_axis = axis;
            }
        }
        if screen_up_mean[switch.screen_normal_axis.index()] < 0.0 {
            switch.screen_up_sign = -1.0;
        }
        switch
    }

    /// Keeps orientation tracking but ignores the flip gesture; modes then
    /// change only through `toggle`.
    pub const fn without_flip(mut self) -> Self {
        self.flip_enabled = false;
        self
    }

    /// Switches mode from another input, if Debug mode is enabled.
    pub fn toggle(&mut self) -> Option<UiMode> {
        if !self.debug_enabled {
            return None;
        }
        self.down_since_ms = None;
        self.flip_armed = false;
        self.release_windows = 0;
        self.mode = match self.mode {
            UiMode::Timer => UiMode::Debug,
            UiMode::Debug => UiMode::Timer,
        };
        Some(self.mode)
    }

    pub const fn mode(&self) -> UiMode {
        self.mode
    }

    pub const fn screen_axis(&self) -> ScreenAxis {
        self.screen_normal_axis
    }

    pub fn screen_direction(&self, mean: [f32; 3]) -> ScreenDirection {
        let vertical = self.screen_vertical(mean);
        if vertical <= -SCREEN_VERTICAL_COS_THRESHOLD {
            ScreenDirection::Down
        } else if vertical >= SCREEN_VERTICAL_COS_THRESHOLD {
            ScreenDirection::Up
        } else if vertical.abs() < SCREEN_VERTICAL_COS_THRESHOLD * 0.25 {
            ScreenDirection::Side
        } else {
            ScreenDirection::Tilted
        }
    }

    /// Uses monotonic milliseconds and averaged motion windows for stable changes.
    pub fn update(&mut self, now_ms: u64, mean: [f32; 3]) -> Option<UiMode> {
        self.last_update_ms = now_ms;
        let raw_vertical = self.screen_relative_vertical(mean);
        let vertical = match self.filtered_vertical {
            Some(filtered) => filtered + ORIENTATION_FILTER_ALPHA * (raw_vertical - filtered),
            None => raw_vertical,
        };
        self.filtered_vertical = Some(vertical);

        if !self.debug_enabled || !self.flip_enabled {
            return None;
        }

        if vertical <= -SCREEN_VERTICAL_COS_THRESHOLD {
            self.release_windows = 0;
            if !self.flip_armed {
                let down_since_ms = *self.down_since_ms.get_or_insert(now_ms);
                if now_ms.saturating_sub(down_since_ms) >= SCREEN_DOWN_HOLD_MS {
                    let mode = self.toggle();
                    // Consume this hold until a stable non-down orientation.
                    self.flip_armed = true;
                    return mode;
                }
            }
            return None;
        }

        self.down_since_ms = None;
        if !self.flip_armed {
            self.release_windows = 0;
            return None;
        }

        self.release_windows = self.release_windows.saturating_add(1);
        if self.release_windows < SCREEN_RELEASE_DEBOUNCE_WINDOWS {
            return None;
        }

        self.flip_armed = false;
        self.release_windows = 0;
        None
    }

    pub fn screen_vertical(&self, mean: [f32; 3]) -> f32 {
        self.filtered_vertical
            .unwrap_or_else(|| self.screen_relative_vertical(mean))
    }

    pub fn screen_raw_vertical(&self, mean: [f32; 3]) -> f32 {
        self.screen_relative_vertical(mean)
    }

    fn screen_relative_vertical(&self, mean: [f32; 3]) -> f32 {
        let Some(direction) = normalized(mean) else {
            return 0.0;
        };
        direction[self.screen_normal_axis.index()] * self.screen_up_sign
    }
}

fn normalized(mean: [f32; 3]) -> Option<[f32; 3]> {
    let magnitude = libm::sqrtf(mean[0] * mean[0] + mean[1] * mean[1] + mean[2] * mean[2]);
    if magnitude <= f32::EPSILON {
        None
    } else {
        Some([
            mean[0] / magnitude,
            mean[1] / magnitude,
            mean[2] / magnitude,
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_up_does_not_change_timer_mode() {
        let mut switch = FlipModeSwitch::with_debug_enabled(UiMode::Timer, true);
        assert_eq!(repeat(&mut switch, 9.8, 10), None);
        assert_eq!(switch.mode(), UiMode::Timer);
    }

    #[test]
    fn held_down_gesture_toggles_once_per_hold() {
        let mut switch = FlipModeSwitch::with_debug_enabled(UiMode::Timer, true);
        assert_eq!(repeat(&mut switch, -9.8, 20), Some(UiMode::Debug));
        assert_eq!(repeat(&mut switch, -9.8, 20), None);
        assert_eq!(switch.mode(), UiMode::Debug);
        assert_eq!(repeat(&mut switch, 9.8, 20), None);
        assert_eq!(repeat(&mut switch, -9.8, 20), Some(UiMode::Timer));
    }

    #[test]
    fn sideways_does_not_change_modes() {
        let mut switch = FlipModeSwitch::with_debug_enabled(UiMode::Timer, true);
        assert_eq!(advance(&mut switch, sample(2.0)), None);
        assert_eq!(advance(&mut switch, sample(0.0)), None);
        assert_eq!(switch.mode(), UiMode::Timer);
    }

    #[test]
    fn reports_the_same_screen_direction_used_for_switching() {
        let switch = FlipModeSwitch::with_debug_enabled(UiMode::Timer, true);
        assert_eq!(switch.screen_direction(sample(-9.8)), ScreenDirection::Down);
        assert_eq!(
            switch.screen_direction([7.0, 0.0, -7.0]),
            ScreenDirection::Down
        );
        assert_eq!(switch.screen_direction(sample(0.0)), ScreenDirection::Side);
        assert_eq!(
            switch.screen_direction([7.0, 0.0, 7.0]),
            ScreenDirection::Up
        );
        assert_eq!(switch.screen_direction(sample(9.8)), ScreenDirection::Up);
    }

    #[test]
    fn transient_orientation_changes_do_not_toggle_or_rearm() {
        let mut switch = FlipModeSwitch::with_debug_enabled(UiMode::Timer, true);
        repeat(&mut switch, 9.8, 5);
        assert_eq!(repeat(&mut switch, -9.8, 3), None);
        advance(&mut switch, sample(9.8));
        assert_eq!(switch.mode(), UiMode::Timer);
        assert_eq!(repeat(&mut switch, -9.8, 20), Some(UiMode::Debug));

        // A brief excursion must not rearm the already-consumed hold.
        assert_eq!(advance(&mut switch, sample(9.8)), None);
        assert_eq!(repeat(&mut switch, -9.8, 20), None);
        assert_eq!(switch.mode(), UiMode::Debug);
    }

    #[test]
    fn one_down_window_is_smoothed_and_does_not_bypass_the_debounce() {
        let mut switch = FlipModeSwitch::with_debug_enabled(UiMode::Timer, true);
        repeat(&mut switch, 9.8, 5);
        advance(&mut switch, sample(-9.8));
        assert_ne!(switch.screen_direction(sample(-9.8)), ScreenDirection::Down);
        assert_eq!(switch.mode(), UiMode::Timer);
    }

    #[test]
    fn tilted_face_down_orientation_is_detected() {
        let mut switch = FlipModeSwitch::with_debug_enabled(UiMode::Timer, true);
        let tilted_down = [6.5, 0.0, -7.5];
        assert_eq!(
            repeat_vector(&mut switch, tilted_down, 20),
            Some(UiMode::Debug)
        );
        assert_eq!(repeat(&mut switch, 9.8, 20), None);
    }

    #[test]
    fn debug_startup_waits_for_a_down_hold() {
        let mut switch = FlipModeSwitch::with_debug_enabled(UiMode::Debug, true);
        assert_eq!(repeat(&mut switch, 9.8, 10), None);
        assert_eq!(switch.mode(), UiMode::Debug);
        assert_eq!(repeat(&mut switch, -9.8, 20), Some(UiMode::Timer));
        assert_eq!(repeat(&mut switch, 9.8, 20), None);
    }

    #[test]
    fn calibration_detects_the_strongest_axis_and_its_sign() {
        let mut switch = FlipModeSwitch::calibrated(UiMode::Timer, [-9.8, 0.2, 0.1]);
        switch.debug_enabled = true;
        assert_eq!(switch.screen_axis(), ScreenAxis::X);
        assert_eq!(
            switch.screen_direction([-9.8, 0.0, 0.0]),
            ScreenDirection::Up
        );
        assert_eq!(
            repeat_vector(&mut switch, [9.8, 0.0, 0.0], 20),
            Some(UiMode::Debug)
        );
        assert_eq!(repeat_vector(&mut switch, [-9.8, 0.0, 0.0], 20), None);
    }

    #[test]
    fn boot_time_tilt_does_not_redefine_the_screen_normal() {
        let switch = FlipModeSwitch::calibrated(UiMode::Timer, [3.0, 0.0, 9.3]);
        assert_eq!(switch.screen_axis(), ScreenAxis::Z);
        assert_eq!(
            switch.screen_direction([9.8, 0.0, 0.0]),
            ScreenDirection::Side
        );
        assert_eq!(
            switch.screen_direction([0.0, 0.0, 9.8]),
            ScreenDirection::Up
        );
    }

    #[test]
    fn calibration_accumulates_a_mean() {
        let mut calibration = OrientationCalibration::new();
        assert_eq!(calibration.mean(), None);
        calibration.add([1.0, 2.0, 3.0]);
        calibration.add([3.0, 4.0, 5.0]);
        assert_eq!(calibration.sample_count(), 2);
        assert_eq!(calibration.mean(), Some([2.0, 3.0, 4.0]));
    }

    #[test]
    fn calibration_requires_enough_samples_near_one_gravity() {
        let mut calibration = OrientationCalibration::new();
        assert_eq!(
            calibration.validate(2),
            Err(CalibrationError::TooFewSamples {
                samples: 0,
                required: 2
            })
        );
        calibration.add([0.5, 0.0, 9.7]);
        assert!(matches!(
            calibration.validate(2),
            Err(CalibrationError::TooFewSamples { samples: 1, .. })
        ));
        calibration.add([-0.5, 0.0, 9.9]);
        let mean = calibration.validate(2).unwrap();
        assert!(mean[0].abs() < 1e-6 && (mean[2] - 9.8).abs() < 1e-5);

        // Zeroed or saturated reads must not pick an arbitrary screen axis.
        let mut zeros = OrientationCalibration::new();
        zeros.add([0.0; 3]);
        assert_eq!(
            zeros.validate(1),
            Err(CalibrationError::NotGravity { magnitude: 0.0 })
        );
        let mut corrupt = OrientationCalibration::new();
        corrupt.add([f32::NAN, 0.0, 9.8]);
        assert!(matches!(
            corrupt.validate(1),
            Err(CalibrationError::NotGravity { .. })
        ));
        let mut saturated = OrientationCalibration::new();
        saturated.add([78.0, 78.0, 78.0]);
        assert!(matches!(
            saturated.validate(1),
            Err(CalibrationError::NotGravity { .. })
        ));
    }

    #[test]
    fn without_flip_ignores_the_gesture_but_toggles_and_tracks_orientation() {
        let mut switch = FlipModeSwitch::with_debug_enabled(UiMode::Timer, true).without_flip();
        repeat(&mut switch, 9.8, 5);
        assert_eq!(repeat(&mut switch, -9.8, 20), None);
        assert_eq!(switch.screen_direction(sample(-9.8)), ScreenDirection::Down);
        assert_eq!(repeat(&mut switch, 9.8, 20), None);
        assert_eq!(switch.mode(), UiMode::Timer);
        assert_eq!(switch.toggle(), Some(UiMode::Debug));
        assert_eq!(switch.toggle(), Some(UiMode::Timer));
    }

    #[test]
    fn toggle_respects_disabled_debug_mode() {
        let mut switch = FlipModeSwitch::with_debug_enabled(UiMode::Timer, false);
        assert_eq!(switch.toggle(), None);
        assert_eq!(switch.mode(), UiMode::Timer);
    }

    fn advance(switch: &mut FlipModeSwitch, mean: [f32; 3]) -> Option<UiMode> {
        switch.update(switch.last_update_ms + 100, mean)
    }

    #[test]
    fn hold_toggles_at_threshold_without_any_prior_up_state() {
        let mut switch = FlipModeSwitch::with_debug_enabled(UiMode::Timer, true);
        let down = sample(-9.8);
        assert_eq!(switch.update(1000, down), None);
        assert_eq!(switch.update(1000 + SCREEN_DOWN_HOLD_MS - 1, down), None);
        assert!(!switch.flip_armed);
        assert_eq!(
            switch.update(1000 + SCREEN_DOWN_HOLD_MS, down),
            Some(UiMode::Debug)
        );
        assert!(switch.flip_armed);
        assert_eq!(switch.mode(), UiMode::Debug);
        assert_eq!(switch.update(1000 + SCREEN_DOWN_HOLD_MS * 10, down), None);
    }

    #[test]
    fn sideways_and_tilted_states_rearm_without_ever_facing_up() {
        for other in [[9.8, 0.0, 0.0], [9.0, 0.0, -4.0]] {
            let mut switch = FlipModeSwitch::with_debug_enabled(UiMode::Timer, true);
            assert_eq!(repeat_vector(&mut switch, other, 20), None);
            assert_eq!(repeat(&mut switch, -9.8, 20), Some(UiMode::Debug));
            assert_eq!(repeat_vector(&mut switch, other, 20), None);
            assert!(!switch.flip_armed);
            assert_eq!(repeat(&mut switch, -9.8, 20), Some(UiMode::Timer));
        }
    }

    #[test]
    fn interrupted_hold_does_not_arm_switching() {
        let mut switch = FlipModeSwitch::with_debug_enabled(UiMode::Timer, true);
        switch.update(0, sample(-9.8));
        switch.update(SCREEN_DOWN_HOLD_MS / 2, sample(-9.8));
        // Sideways interrupts the filtered face-down hold.
        switch.update(SCREEN_DOWN_HOLD_MS / 2 + 1, [9.8, 0.0, 0.0]);
        switch.update(SCREEN_DOWN_HOLD_MS / 2 + 2, [9.8, 0.0, 0.0]);
        assert!(switch.down_since_ms.is_none());
        assert!(!switch.flip_armed);
        assert_eq!(repeat(&mut switch, 9.8, 20), None);
    }

    #[test]
    fn disabling_debug_forces_timer_and_ignores_flips() {
        let mut switch = FlipModeSwitch::with_debug_enabled(UiMode::Debug, false);
        assert_eq!(switch.mode(), UiMode::Timer);
        assert_eq!(repeat(&mut switch, -9.8, 40), None);
        assert_eq!(repeat(&mut switch, 9.8, 40), None);
        assert_eq!(switch.mode(), UiMode::Timer);
        assert!(!switch.flip_armed);
    }

    #[test]
    fn constructor_obeys_debug_mode_setting() {
        let switch = FlipModeSwitch::new(UiMode::Debug);
        let expected = if DEBUG_MODE_ENABLED {
            UiMode::Debug
        } else {
            UiMode::Timer
        };
        assert_eq!(switch.mode(), expected);
    }

    fn repeat(switch: &mut FlipModeSwitch, z: f32, count: usize) -> Option<UiMode> {
        repeat_vector(switch, sample(z), count)
    }

    fn repeat_vector(
        switch: &mut FlipModeSwitch,
        sample: [f32; 3],
        count: usize,
    ) -> Option<UiMode> {
        let mut change = None;
        for _ in 0..count {
            change = advance(switch, sample).or(change);
        }
        change
    }

    fn sample(z: f32) -> [f32; 3] {
        [0.0, 0.0, z]
    }
}
