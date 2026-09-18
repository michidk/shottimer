//! Orientation-controlled switching between Timer and Debug modes.

use crate::settings::{
    ORIENTATION_FILTER_ALPHA, SCREEN_DOWN_DEBOUNCE_WINDOWS, SCREEN_DOWN_RELEASE_COS,
    SCREEN_RELEASE_DEBOUNCE_WINDOWS, SCREEN_UP_Z_SIGN, SCREEN_VERTICAL_COS_THRESHOLD,
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

pub struct FlipModeSwitch {
    mode: UiMode,
    filtered_screen_z: Option<f32>,
    down_windows: u8,
    release_windows: u8,
}

impl FlipModeSwitch {
    pub const fn new(mode: UiMode) -> Self {
        Self {
            mode,
            filtered_screen_z: None,
            down_windows: 0,
            release_windows: 0,
        }
    }

    pub const fn mode(&self) -> UiMode {
        self.mode
    }

    pub fn screen_direction(&self, mean: [f32; 3]) -> ScreenDirection {
        let upright_z = self
            .filtered_screen_z
            .unwrap_or_else(|| screen_relative_z(mean));
        if upright_z <= -SCREEN_VERTICAL_COS_THRESHOLD {
            ScreenDirection::Down
        } else if upright_z >= SCREEN_VERTICAL_COS_THRESHOLD {
            ScreenDirection::Up
        } else if upright_z.abs() <= SCREEN_DOWN_RELEASE_COS.abs() {
            ScreenDirection::Side
        } else {
            ScreenDirection::Tilted
        }
    }

    /// Converts to screen-relative Z, then returns stable mode changes.
    pub fn update(&mut self, mean: [f32; 3]) -> Option<UiMode> {
        let raw_screen_z = screen_relative_z(mean);
        let upright_z = match self.filtered_screen_z {
            Some(filtered) => filtered + ORIENTATION_FILTER_ALPHA * (raw_screen_z - filtered),
            None => raw_screen_z,
        };
        self.filtered_screen_z = Some(upright_z);

        if upright_z > SCREEN_DOWN_RELEASE_COS && raw_screen_z > SCREEN_DOWN_RELEASE_COS {
            self.down_windows = 0;
            if self.mode == UiMode::Debug {
                self.release_windows = self.release_windows.saturating_add(1);
                if self.release_windows >= SCREEN_RELEASE_DEBOUNCE_WINDOWS {
                    self.release_windows = 0;
                    self.mode = UiMode::Timer;
                    return Some(UiMode::Timer);
                }
            }
            return None;
        }

        self.release_windows = 0;
        if upright_z > -SCREEN_VERTICAL_COS_THRESHOLD || raw_screen_z > SCREEN_DOWN_RELEASE_COS {
            self.down_windows = 0;
            return None;
        }
        if self.mode == UiMode::Debug {
            return None;
        }

        self.down_windows = self.down_windows.saturating_add(1);
        if self.down_windows < SCREEN_DOWN_DEBOUNCE_WINDOWS {
            return None;
        }

        self.down_windows = 0;
        self.mode = UiMode::Debug;
        Some(UiMode::Debug)
    }
}

fn screen_relative_z(mean: [f32; 3]) -> f32 {
    let magnitude = libm::sqrtf(mean[0] * mean[0] + mean[1] * mean[1] + mean[2] * mean[2]);
    if magnitude <= f32::EPSILON {
        0.0
    } else {
        mean[2] * SCREEN_UP_Z_SIGN / magnitude
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
    fn screen_down_selects_debug_and_non_down_selects_timer() {
        let mut switch = FlipModeSwitch::new(UiMode::Timer);
        repeat(&mut switch, 9.8, 5);
        assert_eq!(repeat(&mut switch, -9.8, 9), None);
        assert_eq!(repeat(&mut switch, -9.8, 10), Some(UiMode::Debug));
        assert_eq!(repeat(&mut switch, -9.8, 8), None);
        assert_eq!(repeat(&mut switch, 0.0, 4), None);
        assert_eq!(switch.mode(), UiMode::Debug);
        assert_eq!(switch.update(sample(0.0)), Some(UiMode::Timer));
        assert_eq!(repeat(&mut switch, -9.8, 9), None);
        assert_eq!(repeat(&mut switch, -9.8, 5), Some(UiMode::Debug));
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
            switch.screen_direction([9.0, 0.0, -4.0]),
            ScreenDirection::Tilted
        );
        assert_eq!(switch.screen_direction(sample(0.0)), ScreenDirection::Side);
        assert_eq!(
            switch.screen_direction([9.0, 0.0, 4.0]),
            ScreenDirection::Tilted
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
        assert_eq!(repeat(&mut switch, -9.8, 20), Some(UiMode::Debug));

        assert_eq!(repeat(&mut switch, 9.8, 2), None);
        switch.update(sample(-9.8));
        assert_eq!(switch.mode(), UiMode::Debug);
        assert_eq!(repeat(&mut switch, 9.8, 10), Some(UiMode::Timer));
        assert_eq!(repeat(&mut switch, -9.8, 20), Some(UiMode::Debug));
    }

    #[test]
    fn low_pass_filter_rejects_a_single_opposite_sample() {
        let mut switch = FlipModeSwitch::new(UiMode::Timer);
        repeat(&mut switch, 9.8, 5);
        switch.update(sample(-9.8));
        assert_ne!(switch.screen_direction(sample(-9.8)), ScreenDirection::Down);
        assert_eq!(switch.mode(), UiMode::Timer);
    }

    fn repeat(switch: &mut FlipModeSwitch, z: f32, count: usize) -> Option<UiMode> {
        let mut change = None;
        for _ in 0..count {
            change = switch.update(sample(z)).or(change);
        }
        change
    }

    fn sample(z: f32) -> [f32; 3] {
        [0.0, 0.0, z]
    }
}
