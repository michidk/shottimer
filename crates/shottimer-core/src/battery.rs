//! Single-cell LiPo estimation and voltage trends, independent of MCU ADC units.

use crate::settings::{BATTERY_ADC_REFERENCE_VOLTS, BATTERY_VOLTAGE_DIVIDER_RATIO, USE_BATTERY};

const ADC_MAX_COUNT: f32 = 4095.0;
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
    /// Voltage-rise heuristic, not a reading of the charger's inaccessible STAT pin.
    pub fn charging_indicated(&self) -> bool {
        self.connected && self.voltage_trend == VoltageTrend::Rising
    }

    pub fn from_adc_counts(counts: u16) -> Self {
        let voltage = counts as f32 * BATTERY_ADC_REFERENCE_VOLTS
            / ADC_MAX_COUNT
            / BATTERY_VOLTAGE_DIVIDER_RATIO;
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
    battery_enabled: bool,
    filtered_counts: Option<f32>,
    filtered_voltage: Option<f32>,
    baseline_voltage: Option<f32>,
    baseline_ms: u64,
    millivolts_per_minute: i16,
    trend_ready: bool,
}

impl BatteryMonitor {
    pub const fn new() -> Self {
        Self::with_battery_enabled(USE_BATTERY)
    }

    pub const fn with_battery_enabled(battery_enabled: bool) -> Self {
        Self {
            battery_enabled,
            filtered_counts: None,
            filtered_voltage: None,
            baseline_voltage: None,
            baseline_ms: 0,
            millivolts_per_minute: 0,
            trend_ready: false,
        }
    }

    pub fn update(&mut self, counts: u16, now_ms: u64) -> BatteryStatus {
        self.update_voltage(
            counts,
            BatteryStatus::from_adc_counts(counts).voltage,
            now_ms,
        )
    }

    /// Accept a board-calibrated voltage; ADC references/dividers differ by MCU.
    pub fn update_voltage(&mut self, counts: u16, voltage: f32, now_ms: u64) -> BatteryStatus {
        let filtered_counts = match self.filtered_counts {
            Some(filtered) => filtered + FILTER_ALPHA * (counts as f32 - filtered),
            None => counts as f32,
        };
        self.filtered_counts = Some(filtered_counts);
        let mut status = BatteryStatus::from_adc_counts((filtered_counts + 0.5) as u16);
        let voltage = match self.filtered_voltage {
            Some(filtered) => filtered + FILTER_ALPHA * (voltage - filtered),
            None => voltage,
        };
        self.filtered_voltage = Some(voltage);
        status.voltage = voltage;
        status.connected = self.battery_enabled && voltage >= CONNECTED_THRESHOLD_VOLTS;
        status.charge_percent = if status.connected {
            estimate_charge_percent(voltage)
        } else {
            0
        };

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
    // charging or under load will differ; this is not a fuel-gauge measurement.
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
    #[test]
    fn calibrated_board_voltage_does_not_use_rp2040_adc_scaling() {
        let mut monitor = super::BatteryMonitor::with_battery_enabled(true);
        let status = monitor.update_voltage(2800, 3.85, 0);
        assert_eq!(status.raw_counts, 2800);
        assert!((status.voltage - 3.85).abs() < 0.001);
        assert_eq!(status.charge_percent, 50);
        assert!(status.connected);
        let filtered = monitor.update_voltage(3000, 4.10, 100);
        assert!((filtered.voltage - 3.90).abs() < 0.001);
        assert_eq!(filtered.raw_counts, 2840);
    }
    use super::*;

    #[test]
    fn disabled_battery_keeps_adc_diagnostics_without_charge_estimates() {
        let mut monitor = BatteryMonitor::with_battery_enabled(false);
        let status = monitor.update_voltage(2500, 4.0, 0);
        assert!(!status.connected);
        assert_eq!(status.charge_percent, 0);
        assert_eq!(status.raw_counts, 2500);
        assert!((status.voltage - 4.0).abs() < 0.001);
        let rising = monitor.update_voltage(2600, 4.2, TREND_WINDOW_MS);
        assert!(!rising.connected);
        assert!(!rising.charging_indicated());
        assert_eq!(rising.charge_percent, 0);
        assert_eq!(rising.voltage_trend, VoltageTrend::Unknown);
    }

    #[test]
    fn default_monitor_obeys_battery_installation_setting() {
        let status = BatteryMonitor::new().update_voltage(2500, 4.0, 0);
        assert_eq!(status.connected, USE_BATTERY);
    }

    #[test]
    fn charging_indicator_requires_a_connected_battery_and_rising_voltage() {
        let mut status = BatteryStatus::from_adc_counts(2500);
        assert!(!status.charging_indicated());
        status.voltage_trend = VoltageTrend::Rising;
        assert!(status.charging_indicated());
        status.voltage_trend = VoltageTrend::Stable;
        assert!(!status.charging_indicated());
        status.voltage_trend = VoltageTrend::Falling;
        assert!(!status.charging_indicated());
        status.voltage_trend = VoltageTrend::Rising;
        status.connected = false;
        assert!(!status.charging_indicated());
    }

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
        let mut monitor = BatteryMonitor::with_battery_enabled(true);
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
        let mut monitor = BatteryMonitor::with_battery_enabled(true);
        monitor.update(2_500, 0);
        let stable = monitor.update(2_501, TREND_WINDOW_MS);
        assert_eq!(stable.voltage_trend, VoltageTrend::Stable);
    }
}
