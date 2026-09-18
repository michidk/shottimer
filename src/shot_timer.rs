//! Shot detection state machine matching the reference project's timings.

use crate::settings::{COMPLETED_SHOT_HOLD_SECONDS, SHOT_TIMEOUT_SECONDS};

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
    },
    Completed {
        seconds: u64,
        completed_ms: u64,
    },
    TimedOut,
    Sleeping,
}

impl ShotState {
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
}

impl ShotTimer {
    pub const TRIGGER_DELAY_MS: u64 = 2_000;
    pub const SLEEP_TIMEOUT_MS: u64 = 60_000;
    pub const RESULT_HOLD_MS: u64 = COMPLETED_SHOT_HOLD_SECONDS * 1_000;
    pub const TIMER_INCREMENT_MS: u64 = 975;

    pub const fn new(now_ms: u64) -> Self {
        Self {
            state: ShotState::Ready,
            last_activity_ms: now_ms,
        }
    }

    pub const fn state(&self) -> ShotState {
        self.state
    }

    pub fn reset(&mut self, now_ms: u64) -> ShotState {
        self.last_activity_ms = now_ms;
        self.state = ShotState::Ready;
        self.state
    }

    pub fn update(&mut self, now_ms: u64, vibrating: bool) -> ShotState {
        if vibrating {
            self.last_activity_ms = now_ms;
        }

        self.state = match self.state {
            ShotState::Ready if vibrating => ShotState::Confirming {
                first_motion_ms: now_ms,
            },
            ShotState::Ready
                if now_ms.saturating_sub(self.last_activity_ms) >= Self::SLEEP_TIMEOUT_MS =>
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
                    }
                } else {
                    ShotState::Ready
                }
            }

            ShotState::Completed { .. } if vibrating => ShotState::Confirming {
                first_motion_ms: now_ms,
            },
            ShotState::Completed { completed_ms, .. }
                if now_ms.saturating_sub(completed_ms) >= Self::RESULT_HOLD_MS =>
            {
                ShotState::Sleeping
            }
            ShotState::Completed {
                seconds,
                completed_ms,
            } => ShotState::Completed {
                seconds,
                completed_ms,
            },
            ShotState::Confirming { first_motion_ms } => ShotState::Confirming { first_motion_ms },

            ShotState::Timing {
                started_ms,
                bucket_index,
                bucket_saw_vibration,
            } => {
                let current_bucket = now_ms.saturating_sub(started_ms) / Self::TIMER_INCREMENT_MS;
                if Self::displayed_seconds(now_ms, started_ms) >= SHOT_TIMEOUT_SECONDS {
                    ShotState::TimedOut
                } else if current_bucket == bucket_index {
                    ShotState::Timing {
                        started_ms,
                        bucket_index,
                        bucket_saw_vibration: bucket_saw_vibration || vibrating,
                    }
                } else if current_bucket == bucket_index + 1 && bucket_saw_vibration {
                    ShotState::Timing {
                        started_ms,
                        bucket_index: current_bucket,
                        bucket_saw_vibration: vibrating,
                    }
                } else {
                    ShotState::Completed {
                        seconds: Self::displayed_seconds(now_ms, started_ms),
                        completed_ms: now_ms,
                    }
                }
            }

            ShotState::TimedOut if vibrating => ShotState::TimedOut,
            ShotState::TimedOut => {
                self.last_activity_ms = now_ms;
                ShotState::Ready
            }

            ShotState::Sleeping if vibrating => {
                self.last_activity_ms = now_ms;
                ShotState::Ready
            }
            ShotState::Sleeping => ShotState::Sleeping,
        };
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
    fn requires_vibration_in_each_975_ms_bucket() {
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
        assert_eq!(
            timer.update(4_000, false),
            ShotState::Completed {
                seconds: 4,
                completed_ms: 4_000
            }
        );
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
        timer.update(3_900, false);
        assert!(matches!(
            timer.update(4_000, false),
            ShotState::Completed { seconds: 4, .. }
        ));
        assert!(matches!(
            timer.update(63_999, false),
            ShotState::Completed { seconds: 4, .. }
        ));
        assert_eq!(timer.update(64_000, false), ShotState::Sleeping);
    }

    #[test]
    fn new_motion_replaces_the_completed_result() {
        let mut timer = ShotTimer::new(0);
        timer.update(100, true);
        timer.update(2_100, true);
        timer.update(2_900, true);
        timer.update(3_900, false);
        timer.update(4_000, false);
        assert_eq!(
            timer.update(5_000, true),
            ShotState::Confirming {
                first_motion_ms: 5_000
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
}
