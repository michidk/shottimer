//! Battery voltage conversion for the board's GP29 ADC divider.

const ADC_REFERENCE_VOLTS: f32 = 3.3;
const ADC_MAX_COUNT: f32 = 4095.0;
const VOLTAGE_DIVIDER_RATIO: f32 = 0.5;
const CONNECTED_THRESHOLD_VOLTS: f32 = 2.5;
const FILTER_ALPHA: f32 = 0.2;
const TREND_WINDOW_MS: u64 = 30_000;
const TREND_DEADBAND_MV_PER_MINUTE: i16 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoltageTrend {
    Unknown,
    Rising,
    Stable,
    Falling,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BatteryStatus {
    pub raw_counts: u16,
    pub voltage: f32,
    pub connected: bool,
    /// Approximate single-cell LiPo state of charge derived from voltage.
    pub charge_percent: u8,
    pub voltage_trend: VoltageTrend,
    pub millivolts_per_minute: i16,
    pub trend_ready: bool,
}

impl BatteryStatus {
    pub fn from_adc_counts(counts: u16) -> Self {
        let voltage = counts as f32 * ADC_REFERENCE_VOLTS / ADC_MAX_COUNT / VOLTAGE_DIVIDER_RATIO;
        Self {
            raw_counts: counts,
            voltage,
            connected: voltage >= CONNECTED_THRESHOLD_VOLTS,
            charge_percent: estimate_charge_percent(voltage),
            voltage_trend: VoltageTrend::Unknown,
            millivolts_per_minute: 0,
            trend_ready: false,
        }
    }
}

pub struct BatteryMonitor {
    filtered_counts: Option<f32>,
    baseline_voltage: Option<f32>,
    baseline_ms: u64,
    millivolts_per_minute: i16,
    trend_ready: bool,
}

impl BatteryMonitor {
    pub const fn new() -> Self {
        Self {
            filtered_counts: None,
            baseline_voltage: None,
            baseline_ms: 0,
            millivolts_per_minute: 0,
            trend_ready: false,
        }
    }

    pub fn update(&mut self, counts: u16, now_ms: u64) -> BatteryStatus {
        let filtered_counts = match self.filtered_counts {
            Some(filtered) => filtered + FILTER_ALPHA * (counts as f32 - filtered),
            None => counts as f32,
        };
        self.filtered_counts = Some(filtered_counts);
        let mut status = BatteryStatus::from_adc_counts((filtered_counts + 0.5) as u16);

        if !status.connected {
            self.baseline_voltage = None;
            self.millivolts_per_minute = 0;
            self.trend_ready = false;
        } else if let Some(baseline_voltage) = self.baseline_voltage {
            let elapsed_ms = now_ms.saturating_sub(self.baseline_ms);
            if elapsed_ms >= TREND_WINDOW_MS {
                let rate = (status.voltage - baseline_voltage) * 60_000_000.0 / elapsed_ms as f32;
                self.millivolts_per_minute =
                    libm::roundf(rate.clamp(i16::MIN as f32, i16::MAX as f32)) as i16;
                self.trend_ready = true;
                self.baseline_voltage = Some(status.voltage);
                self.baseline_ms = now_ms;
            }
        } else {
            self.baseline_voltage = Some(status.voltage);
            self.baseline_ms = now_ms;
        }

        status.millivolts_per_minute = self.millivolts_per_minute;
        status.trend_ready = self.trend_ready;
        status.voltage_trend = if !self.trend_ready {
            VoltageTrend::Unknown
        } else if self.millivolts_per_minute > TREND_DEADBAND_MV_PER_MINUTE {
            VoltageTrend::Rising
        } else if self.millivolts_per_minute < -TREND_DEADBAND_MV_PER_MINUTE {
            VoltageTrend::Falling
        } else {
            VoltageTrend::Stable
        };
        status
    }
}

impl Default for BatteryMonitor {
    fn default() -> Self {
        Self::new()
    }
}

fn estimate_charge_percent(voltage: f32) -> u8 {
    // Approximate resting-voltage curve for a single-cell LiPo. Readings while
    // charging or under load will differ, so the UI marks this value with `~`.
    const CURVE: [(f32, u8); 12] = [
        (3.20, 0),
        (3.50, 5),
        (3.60, 10),
        (3.70, 20),
        (3.75, 30),
        (3.80, 40),
        (3.85, 50),
        (3.90, 60),
        (3.95, 70),
        (4.00, 80),
        (4.10, 90),
        (4.20, 100),
    ];

    if voltage <= CURVE[0].0 {
        return 0;
    }

    for pair in CURVE.windows(2) {
        let (low_voltage, low_percent) = pair[0];
        let (high_voltage, high_percent) = pair[1];
        if voltage <= high_voltage {
            let position = (voltage - low_voltage) / (high_voltage - low_voltage);
            let percent = low_percent as f32 + position * (high_percent - low_percent) as f32;
            return (percent + 0.5) as u8;
        }
    }

    100
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_the_board_voltage_divider() {
        let status = BatteryStatus::from_adc_counts(2_544);
        assert!((status.voltage - 4.1).abs() < 0.01);
        assert_eq!(status.raw_counts, 2_544);
        assert!(status.connected);
        assert_eq!(status.charge_percent, 90);
    }

    #[test]
    fn low_floating_voltage_means_no_battery() {
        let status = BatteryStatus::from_adc_counts(100);
        assert!(!status.connected);
        assert_eq!(status.charge_percent, 0);
    }

    #[test]
    fn interpolates_single_cell_lipo_charge() {
        assert_eq!(estimate_charge_percent(3.775), 35);
        assert_eq!(estimate_charge_percent(4.30), 100);
    }

    #[test]
    fn measures_sustained_voltage_trends() {
        let mut monitor = BatteryMonitor::new();
        monitor.update(2_420, 0);
        let rising = monitor.update(2_450, TREND_WINDOW_MS);
        assert_eq!(rising.voltage_trend, VoltageTrend::Rising);
        assert!(rising.millivolts_per_minute > 0);

        let falling = monitor.update(2_400, TREND_WINDOW_MS * 2);
        assert_eq!(falling.voltage_trend, VoltageTrend::Falling);
        assert!(falling.millivolts_per_minute < 0);
    }

    #[test]
    fn ignores_small_voltage_changes() {
        let mut monitor = BatteryMonitor::new();
        monitor.update(2_500, 0);
        let stable = monitor.update(2_501, TREND_WINDOW_MS);
        assert_eq!(stable.voltage_trend, VoltageTrend::Stable);
    }
}
