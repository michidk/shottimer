//! Shot detection state machine matching the reference project's timings.

use crate::settings::{
    COMPLETED_SHOT_HOLD_SECONDS, MINIMUM_SHOT_SECONDS, RESTART_CONFIRM_SECONDS,
    SHOT_TIMEOUT_SECONDS, SLEEP_TIMEOUT_SECONDS, START_CONFIRM_SECONDS,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShotState {
    Ready,
    Confirming {
        first_motion_ms: u64,
    },
    Timing {
        started_ms: u64,
        bucket_index: u64,
        bucket_saw_vibration: bool,
        /// End of the most recent vibrating sample window, excluding stop debounce.
        last_vibration_ms: u64,
    },
    Completed {
        seconds: u64,
        completed_ms: u64,
    },
    RestartConfirming {
        first_motion_ms: u64,
        retained_seconds: u64,
        retained_completed_ms: u64,
    },
    TimedOut,
    Sleeping,
}

impl ShotState {
    /// Wall-clock time until the next confirmation, result expiry, or timeout.
    pub fn countdown_ms(self, now_ms: u64) -> Option<u64> {
        let (started_ms, duration_ms) = match self {
            Self::Confirming { first_motion_ms } => (first_motion_ms, ShotTimer::TRIGGER_DELAY_MS),
            Self::RestartConfirming {
                first_motion_ms, ..
            } => (first_motion_ms, ShotTimer::RESTART_CONFIRM_MS),
            Self::Timing { started_ms, .. } => (
                started_ms,
                SHOT_TIMEOUT_SECONDS * ShotTimer::TIMER_INCREMENT_MS,
            ),
            Self::Completed { completed_ms, .. } => (completed_ms, ShotTimer::RESULT_HOLD_MS),
            _ => return None,
        };
        Some(duration_ms.saturating_sub(now_ms.saturating_sub(started_ms)))
    }

    pub const fn is_timing(self) -> bool {
        matches!(self, Self::Timing { .. })
    }

    pub const fn is_sleeping(self) -> bool {
        matches!(self, Self::Sleeping)
    }
}

pub struct ShotTimer {
    state: ShotState,
    last_activity_ms: u64,
    // Current completed shot followed by up to three previous valid shots.
    recent_shots: [Option<u64>; 4],
}

impl ShotTimer {
    pub const TRIGGER_DELAY_MS: u64 = START_CONFIRM_SECONDS * 1_000;
    pub const SLEEP_TIMEOUT_MS: u64 = SLEEP_TIMEOUT_SECONDS * 1_000;
    pub const RESULT_HOLD_MS: u64 = COMPLETED_SHOT_HOLD_SECONDS * 1_000;
    pub const RESTART_CONFIRM_MS: u64 = RESTART_CONFIRM_SECONDS * 1_000;
    pub const TIMER_INCREMENT_MS: u64 = 975;

    pub const fn new(now_ms: u64) -> Self {
        Self {
            state: ShotState::Ready,
            last_activity_ms: now_ms,
            recent_shots: [None; 4],
        }
    }

    pub const fn state(&self) -> ShotState {
        self.state
    }

    /// Previous results, newest first. Exclude the current result only while
    /// it is already displayed as the main retained number.
    /// Timer resets and UI changes preserve history; a reboot clears it.
    pub fn previous_shots(&self) -> [Option<u64>; 3] {
        let offset = usize::from(matches!(
            self.state,
            ShotState::Completed { .. } | ShotState::RestartConfirming { .. }
        ));
        [
            self.recent_shots[offset],
            self.recent_shots[offset + 1],
            self.recent_shots[offset + 2],
        ]
    }

    pub fn reset(&mut self, now_ms: u64) -> ShotState {
        self.last_activity_ms = now_ms;
        self.state = ShotState::Ready;
        self.state
    }

    pub fn update(&mut self, now_ms: u64, vibrating: bool) -> ShotState {
        self.update_with_sleep_policy(now_ms, vibrating, true)
    }

    /// Disable sleep while externally powered/charging without losing shot state.
    pub fn update_with_sleep_policy(
        &mut self,
        now_ms: u64,
        vibrating: bool,
        allow_sleep: bool,
    ) -> ShotState {
        let was_timing = self.state.is_timing();
        if vibrating {
            self.last_activity_ms = now_ms;
        }

        self.state = match self.state {
            ShotState::Ready if vibrating => ShotState::Confirming {
                first_motion_ms: now_ms,
            },
            ShotState::Ready
                if allow_sleep
                    && now_ms.saturating_sub(self.last_activity_ms) >= Self::SLEEP_TIMEOUT_MS =>
            {
                ShotState::Sleeping
            }
            ShotState::Ready => ShotState::Ready,

            ShotState::Confirming { first_motion_ms }
                if now_ms.saturating_sub(first_motion_ms) >= Self::TRIGGER_DELAY_MS =>
            {
                if vibrating {
                    ShotState::Timing {
                        started_ms: first_motion_ms,
                        bucket_index: now_ms.saturating_sub(first_motion_ms)
                            / Self::TIMER_INCREMENT_MS,
                        bucket_saw_vibration: true,
                        last_vibration_ms: now_ms,
                    }
                } else {
                    ShotState::Ready
                }
            }

            ShotState::Completed {
                seconds,
                completed_ms,
            } if vibrating => ShotState::RestartConfirming {
                first_motion_ms: now_ms,
                retained_seconds: seconds,
                retained_completed_ms: completed_ms,
            },
            ShotState::Completed { completed_ms, .. }
                if now_ms.saturating_sub(completed_ms) >= Self::RESULT_HOLD_MS =>
            {
                if allow_sleep {
                    ShotState::Sleeping
                } else {
                    ShotState::Ready
                }
            }
            ShotState::Completed {
                seconds,
                completed_ms,
            } => ShotState::Completed {
                seconds,
                completed_ms,
            },
            ShotState::Confirming { first_motion_ms } => ShotState::Confirming { first_motion_ms },

            ShotState::RestartConfirming {
                first_motion_ms,
                retained_seconds,
                retained_completed_ms,
            } if now_ms.saturating_sub(first_motion_ms) >= Self::RESTART_CONFIRM_MS => {
                if vibrating {
                    ShotState::Timing {
                        started_ms: first_motion_ms,
                        bucket_index: now_ms.saturating_sub(first_motion_ms)
                            / Self::TIMER_INCREMENT_MS,
                        bucket_saw_vibration: true,
                        last_vibration_ms: now_ms,
                    }
                } else if now_ms.saturating_sub(retained_completed_ms) >= Self::RESULT_HOLD_MS {
                    if allow_sleep {
                        ShotState::Sleeping
                    } else {
                        ShotState::Ready
                    }
                } else {
                    ShotState::Completed {
                        seconds: retained_seconds,
                        completed_ms: retained_completed_ms,
                    }
                }
            }
            ShotState::RestartConfirming {
                retained_seconds,
                retained_completed_ms,
                ..
            } if !vibrating => ShotState::Completed {
                seconds: retained_seconds,
                completed_ms: retained_completed_ms,
            },
            ShotState::RestartConfirming {
                first_motion_ms,
                retained_seconds,
                retained_completed_ms,
            } => ShotState::RestartConfirming {
                first_motion_ms,
                retained_seconds,
                retained_completed_ms,
            },

            ShotState::Timing {
                started_ms,
                bucket_index,
                bucket_saw_vibration,
                last_vibration_ms,
            } => {
                let current_bucket = now_ms.saturating_sub(started_ms) / Self::TIMER_INCREMENT_MS;
                if Self::displayed_seconds(now_ms, started_ms) >= SHOT_TIMEOUT_SECONDS {
                    ShotState::TimedOut
                } else if current_bucket == bucket_index {
                    ShotState::Timing {
                        started_ms,
                        bucket_index,
                        bucket_saw_vibration: bucket_saw_vibration || vibrating,
                        last_vibration_ms: if vibrating { now_ms } else { last_vibration_ms },
                    }
                } else if current_bucket == bucket_index + 1 && bucket_saw_vibration {
                    ShotState::Timing {
                        started_ms,
                        bucket_index: current_bucket,
                        bucket_saw_vibration: vibrating,
                        last_vibration_ms: if vibrating { now_ms } else { last_vibration_ms },
                    }
                } else {
                    // Detect stopping conservatively, but retain the actual last
                    // vibration time instead of counting the quiet confirmation.
                    let seconds = Self::displayed_seconds(last_vibration_ms, started_ms);
                    if seconds < MINIMUM_SHOT_SECONDS {
                        self.last_activity_ms = now_ms;
                        ShotState::Ready
                    } else {
                        ShotState::Completed {
                            seconds,
                            completed_ms: now_ms,
                        }
                    }
                }
            }

            ShotState::TimedOut if vibrating => ShotState::TimedOut,
            ShotState::TimedOut => {
                self.last_activity_ms = now_ms;
                ShotState::Ready
            }

            ShotState::Sleeping if vibrating || !allow_sleep => {
                self.last_activity_ms = now_ms;
                ShotState::Ready
            }
            ShotState::Sleeping => ShotState::Sleeping,
        };
        if was_timing && let ShotState::Completed { seconds, .. } = self.state {
            self.recent_shots.rotate_right(1);
            self.recent_shots[0] = Some(seconds);
        }
        self.state
    }

    pub fn displayed_seconds(now_ms: u64, started_ms: u64) -> u64 {
        now_ms.saturating_sub(started_ms) / Self::TIMER_INCREMENT_MS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_power_prevents_sleep_without_extending_result_retention() {
        let mut timer = ShotTimer::new(0);
        assert_eq!(
            timer.update_with_sleep_policy(60_000, false, false),
            ShotState::Ready
        );
        let mut timer = completed_timer();
        assert!(matches!(
            timer.update_with_sleep_policy(66_999, false, false),
            ShotState::Completed { .. }
        ));
        assert_eq!(
            timer.update_with_sleep_policy(67_000, false, false),
            ShotState::Ready
        );
        assert_eq!(timer.previous_shots(), [Some(5), None, None]);
        assert_eq!(
            timer.update_with_sleep_policy(67_100, false, true),
            ShotState::Sleeping
        );
        assert_eq!(
            timer.update_with_sleep_policy(67_200, false, false),
            ShotState::Ready
        );
    }

    #[test]
    fn history_keeps_three_previous_valid_results_without_duplicates() {
        let mut timer = ShotTimer::new(0);
        for seconds in [10, 20, 30, 40, 50] {
            finish_shot(&mut timer, seconds);
        }
        assert_eq!(timer.previous_shots(), [Some(40), Some(30), Some(20)]);
        let completed_ms = 52 * ShotTimer::TIMER_INCREMENT_MS;
        timer.update(completed_ms + 100, true);
        timer.update(completed_ms + 200, false);
        assert_eq!(timer.previous_shots(), [Some(40), Some(30), Some(20)]);
        timer.reset(completed_ms + 300);
        assert_eq!(timer.previous_shots(), [Some(50), Some(40), Some(30)]);
        finish_shot(&mut timer, 4); // Discarded, despite the detection delay.
        assert_eq!(timer.previous_shots(), [Some(50), Some(40), Some(30)]);
        finish_shot(&mut timer, 25);
        assert_eq!(timer.previous_shots(), [Some(50), Some(40), Some(30)]);
    }

    fn finish_shot(timer: &mut ShotTimer, seconds: u64) {
        timer.state = ShotState::Timing {
            started_ms: 0,
            bucket_index: seconds,
            bucket_saw_vibration: true,
            last_vibration_ms: seconds * ShotTimer::TIMER_INCREMENT_MS,
        };
        timer.update((seconds + 2) * ShotTimer::TIMER_INCREMENT_MS, false);
    }

    #[test]
    fn final_time_excludes_quiet_detection_delay() {
        for detection_ms in [26_500, 27_500] {
            let mut timer = ShotTimer::new(100);
            timer.state = ShotState::Timing {
                started_ms: 100,
                bucket_index: 25,
                bucket_saw_vibration: true,
                last_vibration_ms: 25_100,
            };
            timer.update(26_000, false);
            assert_eq!(
                timer.update(detection_ms, false),
                ShotState::Completed {
                    seconds: 25,
                    completed_ms: detection_ms,
                }
            );
            // Retention starts when the corrected result becomes visible.
            assert_eq!(
                timer.state().countdown_ms(detection_ms),
                Some(ShotTimer::RESULT_HOLD_MS)
            );
        }
    }

    #[test]
    fn stop_delay_cannot_make_a_short_shot_pass_minimum_duration() {
        let mut timer = ShotTimer::new(100);
        timer.state = ShotState::Timing {
            started_ms: 100,
            bucket_index: 4,
            bucket_saw_vibration: true,
            last_vibration_ms: 4_800,
        };
        timer.update(5_800, false);
        assert_eq!(timer.update(6_000, false), ShotState::Ready);
    }

    #[test]
    fn countdowns_match_state_transition_deadlines() {
        let confirming = ShotState::Confirming {
            first_motion_ms: 100,
        };
        assert_eq!(confirming.countdown_ms(600), Some(1_500));
        assert_eq!(confirming.countdown_ms(2_200), Some(0));
        let completed = ShotState::Completed {
            seconds: 25,
            completed_ms: 10_000,
        };
        assert_eq!(completed.countdown_ms(11_000), Some(59_000));
        let restarting = ShotState::RestartConfirming {
            first_motion_ms: 20_000,
            retained_seconds: 25,
            retained_completed_ms: 10_000,
        };
        assert_eq!(restarting.countdown_ms(21_000), Some(2_000));
        let timing = ShotState::Timing {
            started_ms: 100,
            bucket_index: 0,
            bucket_saw_vibration: true,
            last_vibration_ms: 100,
        };
        assert_eq!(
            timing.countdown_ms(100),
            Some(SHOT_TIMEOUT_SECONDS * ShotTimer::TIMER_INCREMENT_MS)
        );
        assert_eq!(ShotState::Ready.countdown_ms(100), None);
    }

    #[test]
    fn requires_motion_again_after_trigger_delay() {
        let mut timer = ShotTimer::new(0);
        assert!(matches!(
            timer.update(100, true),
            ShotState::Confirming { .. }
        ));
        assert!(matches!(
            timer.update(2_099, true),
            ShotState::Confirming { .. }
        ));
        assert!(matches!(timer.update(2_100, false), ShotState::Ready));
    }

    #[test]
    fn discards_a_shot_shorter_than_five_seconds() {
        let mut timer = ShotTimer::new(0);
        timer.update(100, true);
        assert!(matches!(
            timer.update(2_100, true),
            ShotState::Timing { .. }
        ));
        timer.update(2_900, true);
        assert!(matches!(
            timer.update(3_900, false),
            ShotState::Timing { .. }
        ));
        assert_eq!(timer.update(4_000, false), ShotState::Ready);
    }

    #[test]
    fn one_positive_window_is_enough_for_a_bucket() {
        let mut timer = ShotTimer::new(0);
        timer.update(100, true);
        timer.update(2_100, true);
        timer.update(2_200, false);
        timer.update(3_030, false);
        timer.update(3_500, true);
        assert!(matches!(
            timer.update(4_000, false),
            ShotState::Timing { .. }
        ));
    }

    #[test]
    fn sleeps_and_wakes_without_starting_a_shot() {
        let mut timer = ShotTimer::new(0);
        assert_eq!(timer.update(60_000, false), ShotState::Sleeping);
        assert_eq!(timer.update(61_000, true), ShotState::Ready);
    }

    #[test]
    fn reset_cancels_an_active_shot() {
        let mut timer = ShotTimer::new(0);
        timer.update(100, true);
        timer.update(2_100, true);
        assert!(timer.state().is_timing());
        assert_eq!(timer.reset(2_500), ShotState::Ready);
    }

    #[test]
    fn holds_the_completed_time_for_sixty_seconds() {
        let mut timer = ShotTimer::new(0);
        timer.update(100, true);
        timer.update(2_100, true);
        timer.update(2_900, true);
        timer.update(3_900, true);
        timer.update(4_800, true);
        timer.update(5_100, true);
        timer.update(5_800, false);
        timer.update(6_000, false);
        assert!(matches!(
            timer.update(7_000, false),
            ShotState::Completed { seconds: 5, .. }
        ));
        assert!(matches!(
            timer.update(66_999, false),
            ShotState::Completed { seconds: 5, .. }
        ));
        assert_eq!(timer.update(67_000, false), ShotState::Sleeping);
    }

    #[test]
    fn new_motion_replaces_the_completed_result_after_three_seconds() {
        let mut timer = completed_timer();
        assert!(matches!(
            timer.update(8_000, true),
            ShotState::RestartConfirming {
                first_motion_ms: 8_000,
                retained_seconds: 5,
                ..
            }
        ));
        assert!(matches!(
            timer.update(10_999, true),
            ShotState::RestartConfirming {
                retained_seconds: 5,
                ..
            }
        ));
        assert!(matches!(
            timer.update(11_000, true),
            ShotState::Timing {
                started_ms: 8_000,
                ..
            }
        ));
        assert_eq!(ShotTimer::displayed_seconds(11_000, 8_000), 3);
    }

    #[test]
    fn brief_motion_does_not_clear_the_completed_result() {
        let mut timer = completed_timer();
        timer.update(8_000, true);
        assert_eq!(
            timer.update(10_000, false),
            ShotState::Completed {
                seconds: 5,
                completed_ms: 7_000,
            }
        );
    }

    #[test]
    fn active_shot_times_out_at_the_configured_limit() {
        let mut timer = ShotTimer::new(0);
        timer.update(0, true);
        timer.update(2_000, true);
        for bucket in 3..SHOT_TIMEOUT_SECONDS {
            assert!(matches!(
                timer.update(bucket * ShotTimer::TIMER_INCREMENT_MS, true),
                ShotState::Timing { .. }
            ));
        }
        assert_eq!(
            timer.update(SHOT_TIMEOUT_SECONDS * ShotTimer::TIMER_INCREMENT_MS, true),
            ShotState::TimedOut
        );
        assert_eq!(timer.update(100_000, true), ShotState::TimedOut);
        assert_eq!(timer.update(100_100, false), ShotState::Ready);
    }

    fn completed_timer() -> ShotTimer {
        let mut timer = ShotTimer::new(0);
        timer.update(100, true);
        timer.update(2_100, true);
        timer.update(2_900, true);
        timer.update(3_900, true);
        timer.update(4_800, true);
        timer.update(5_100, true);
        timer.update(5_800, false);
        timer.update(6_000, false);
        assert!(matches!(
            timer.update(7_000, false),
            ShotState::Completed { seconds: 5, .. }
        ));
        timer
    }
}
