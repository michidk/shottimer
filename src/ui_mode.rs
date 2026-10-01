//! Orientation-controlled switching between Timer and Debug modes.

use crate::settings::{
    ORIENTATION_FILTER_ALPHA, SCREEN_DOWN_DEBOUNCE_WINDOWS, SCREEN_RELEASE_DEBOUNCE_WINDOWS,
    SCREEN_VERTICAL_COS_THRESHOLD,
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
}

pub struct FlipModeSwitch {
    mode: UiMode,
    filtered_vertical: Option<f32>,
    down_windows: u8,
    release_windows: u8,
    flip_armed: bool,
    screen_normal_axis: ScreenAxis,
    screen_up_sign: f32,
}

impl FlipModeSwitch {
    pub const fn new(mode: UiMode) -> Self {
        Self {
            mode,
            filtered_vertical: None,
            down_windows: 0,
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

    /// Uses each averaged 100 ms motion window to return stable mode changes.
    pub fn update(&mut self, mean: [f32; 3]) -> Option<UiMode> {
        let raw_vertical = self.screen_relative_vertical(mean);
        let vertical = match self.filtered_vertical {
            Some(filtered) => filtered + ORIENTATION_FILTER_ALPHA * (raw_vertical - filtered),
            None => raw_vertical,
        };
        self.filtered_vertical = Some(vertical);

        if vertical <= -SCREEN_VERTICAL_COS_THRESHOLD {
            self.release_windows = 0;
            if !self.flip_armed {
                self.down_windows = self.down_windows.saturating_add(1);
                if self.down_windows >= SCREEN_DOWN_DEBOUNCE_WINDOWS {
                    self.down_windows = 0;
                    self.flip_armed = true;
                }
            }
            return None;
        }

        self.down_windows = 0;
        if !self.flip_armed || vertical < SCREEN_VERTICAL_COS_THRESHOLD {
            self.release_windows = 0;
            return None;
        }

        self.release_windows = self.release_windows.saturating_add(1);
        if self.release_windows < SCREEN_RELEASE_DEBOUNCE_WINDOWS {
            return None;
        }

        self.release_windows = 0;
        self.flip_armed = false;
        self.mode = match self.mode {
            UiMode::Timer => UiMode::Debug,
            UiMode::Debug => UiMode::Timer,
        };
        Some(self.mode)
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
        let mut switch = FlipModeSwitch::new(UiMode::Timer);
        assert_eq!(repeat(&mut switch, 9.8, 10), None);
        assert_eq!(switch.mode(), UiMode::Timer);
    }

    #[test]
    fn held_down_then_up_gesture_toggles_both_modes() {
        let mut switch = FlipModeSwitch::new(UiMode::Timer);
        repeat(&mut switch, 9.8, 5);
        assert_eq!(repeat(&mut switch, -9.8, 9), None);
        assert_eq!(repeat(&mut switch, -9.8, 20), None);
        assert_eq!(switch.mode(), UiMode::Timer);
        assert_eq!(repeat(&mut switch, 9.8, 2), None);
        assert_eq!(repeat(&mut switch, 9.8, 20), Some(UiMode::Debug));
        assert_eq!(repeat(&mut switch, -9.8, 20), None);
        assert_eq!(repeat(&mut switch, 9.8, 20), Some(UiMode::Timer));
    }

    #[test]
    fn sideways_does_not_change_modes() {
        let mut switch = FlipModeSwitch::new(UiMode::Timer);
        assert_eq!(switch.update(sample(2.0)), None);
        assert_eq!(switch.update(sample(0.0)), None);
        assert_eq!(switch.mode(), UiMode::Timer);
    }

    #[test]
    fn reports_the_same_screen_direction_used_for_switching() {
        let switch = FlipModeSwitch::new(UiMode::Timer);
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
        let mut switch = FlipModeSwitch::new(UiMode::Timer);
        repeat(&mut switch, 9.8, 5);
        assert_eq!(repeat(&mut switch, -9.8, 3), None);
        switch.update(sample(9.8));
        assert_eq!(switch.mode(), UiMode::Timer);
        assert_eq!(repeat(&mut switch, -9.8, 20), None);

        assert_eq!(repeat(&mut switch, 9.8, 2), None);
        switch.update(sample(-9.8));
        assert_eq!(switch.mode(), UiMode::Timer);
        assert_eq!(repeat(&mut switch, 9.8, 20), Some(UiMode::Debug));
    }

    #[test]
    fn one_down_window_is_smoothed_and_does_not_bypass_the_debounce() {
        let mut switch = FlipModeSwitch::new(UiMode::Timer);
        repeat(&mut switch, 9.8, 5);
        switch.update(sample(-9.8));
        assert_ne!(switch.screen_direction(sample(-9.8)), ScreenDirection::Down);
        assert_eq!(switch.mode(), UiMode::Timer);
    }

    #[test]
    fn tilted_face_down_orientation_is_detected() {
        let mut switch = FlipModeSwitch::new(UiMode::Timer);
        let tilted_down = [6.5, 0.0, -7.5];
        for _ in 0..20 {
            assert_eq!(switch.update(tilted_down), None);
        }
        assert_eq!(repeat(&mut switch, 9.8, 20), Some(UiMode::Debug));
    }

    #[test]
    fn debug_startup_waits_for_a_down_then_up_cycle() {
        let mut switch = FlipModeSwitch::new(UiMode::Debug);
        assert_eq!(repeat(&mut switch, 9.8, 10), None);
        assert_eq!(switch.mode(), UiMode::Debug);
        assert_eq!(repeat(&mut switch, -9.8, 20), None);
        assert_eq!(repeat(&mut switch, 9.8, 20), Some(UiMode::Timer));
    }

    #[test]
    fn calibration_detects_the_strongest_axis_and_its_sign() {
        let mut switch = FlipModeSwitch::calibrated(UiMode::Timer, [-9.8, 0.2, 0.1]);
        assert_eq!(switch.screen_axis(), ScreenAxis::X);
        assert_eq!(
            switch.screen_direction([-9.8, 0.0, 0.0]),
            ScreenDirection::Up
        );
        for _ in 0..20 {
            assert_eq!(switch.update([9.8, 0.0, 0.0]), None);
        }
        assert_eq!(
            repeat_vector(&mut switch, [-9.8, 0.0, 0.0], 20),
            Some(UiMode::Debug)
        );
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
            change = switch.update(sample).or(change);
        }
        change
    }

    fn sample(z: f32) -> [f32; 3] {
        [0.0, 0.0, z]
    }
}
