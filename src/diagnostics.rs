//! Small, platform-independent vibration statistics used by the debug screen.

/// Number of accelerometer readings in one diagnostic window.
pub const SAMPLE_COUNT: usize = 10;

/// Statistics for one sample window, expressed in m/s².
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionStats {
    pub mean: [f32; 3],
    pub deviation: [f32; 3],
    pub peak_deviation: f32,
}

/// Rolling maximum that keeps short vibration events readable on the LCD.
pub struct PeakWindow<const N: usize> {
    values: [f32; N],
    next: usize,
}

impl<const N: usize> PeakWindow<N> {
    pub const fn new() -> Self {
        assert!(N > 0);
        Self {
            values: [0.0; N],
            next: 0,
        }
    }

    pub fn push(&mut self, value: f32) -> f32 {
        self.values[self.next] = value;
        self.next = (self.next + 1) % N;
        self.values.iter().copied().fold(0.0, f32::max)
    }
}

impl<const N: usize> Default for PeakWindow<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl MotionStats {
    /// Calculates population standard deviation independently for all axes.
    pub fn from_samples(samples: &[[f32; 3]; SAMPLE_COUNT]) -> Self {
        let mut mean = [0.0; 3];
        for sample in samples {
            for axis in 0..3 {
                mean[axis] += sample[axis];
            }
        }
        for value in &mut mean {
            *value /= SAMPLE_COUNT as f32;
        }

        let mut deviation = [0.0; 3];
        for sample in samples {
            for axis in 0..3 {
                let delta = sample[axis] - mean[axis];
                deviation[axis] += delta * delta;
            }
        }
        for value in &mut deviation {
            *value = libm::sqrtf(*value / SAMPLE_COUNT as f32);
        }

        let peak_deviation = deviation[0].max(deviation[1]).max(deviation[2]);
        Self {
            mean,
            deviation,
            peak_deviation,
        }
    }

    pub fn is_vibrating(&self, threshold: f32) -> bool {
        self.peak_deviation > threshold
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;

    #[test]
    fn constant_acceleration_is_quiet() {
        let samples = [[0.0, 0.0, 9.81]; SAMPLE_COUNT];
        let stats = MotionStats::from_samples(&samples);
        assert!(stats.deviation.iter().all(|value| *value < 0.000_01));
        assert!(!stats.is_vibrating(0.5));
    }

    #[test]
    fn detects_variation_on_any_axis() {
        let mut samples = [[0.0, 0.0, 9.81]; SAMPLE_COUNT];
        for (index, sample) in samples.iter_mut().enumerate() {
            sample[1] = if index % 2 == 0 { -1.0 } else { 1.0 };
        }
        let stats = MotionStats::from_samples(&samples);
        assert!((stats.deviation[1] - 1.0).abs() < 0.001);
        assert!(stats.is_vibrating(0.5));
    }

    #[test]
    fn rolling_peak_expires_old_values() {
        let mut peaks = PeakWindow::<3>::new();
        assert_eq!(peaks.push(2.0), 2.0);
        assert_eq!(peaks.push(1.0), 2.0);
        assert_eq!(peaks.push(0.5), 2.0);
        assert_eq!(peaks.push(0.25), 1.0);
    }
}
