//! Battery voltage conversion for the board's GP29 ADC divider.

const ADC_REFERENCE_VOLTS: f32 = 3.3;
const ADC_MAX_COUNT: f32 = 4095.0;
const VOLTAGE_DIVIDER_RATIO: f32 = 0.5;
const CONNECTED_THRESHOLD_VOLTS: f32 = 2.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BatteryStatus {
    pub voltage: f32,
    pub connected: bool,
    /// Approximate single-cell LiPo state of charge derived from voltage.
    pub charge_percent: u8,
}

impl BatteryStatus {
    pub fn from_adc_counts(counts: u16) -> Self {
        let voltage = counts as f32 * ADC_REFERENCE_VOLTS / ADC_MAX_COUNT / VOLTAGE_DIVIDER_RATIO;
        Self {
            voltage,
            connected: voltage >= CONNECTED_THRESHOLD_VOLTS,
            charge_percent: estimate_charge_percent(voltage),
        }
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
}
