//! Board boundary: peripheral setup, ADC conversion, power, and logging stay
//! outside the application. Battery readings use volts; IMU samples use the
//! shared driver's signed, decoded ±8 g counts.

pub type Region = (u16, u16, u32, u32);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BatteryReading {
    pub raw_counts: u16,
    pub voltage: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WakeStatus {
    /// Raw pin level for diagnostics, not a confirmed motion event.
    pub interrupt_high: bool,
    pub motion_detected: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HardwareError {
    Imu,
    ImuCommandTimeout,
    Display,
    Battery,
    Sleep,
    Touch,
}

pub trait Platform {
    /// Monotonic uptime in milliseconds, including light-sleep intervals.
    fn now_ms(&self) -> u64;
    fn delay_ms(&mut self, ms: u32);
    /// Reset and configure the LCD controller. Called at boot and again after
    /// repeated transfer failures, so it must be safe to repeat.
    fn initialize_display(&mut self) -> Result<(), HardwareError>;
    fn initialize_imu(&mut self) -> Result<u8, HardwareError>;
    /// Signed QMI8658 counts in ±8 g mode, not register bytes.
    fn read_accel_raw(&mut self) -> Result<[i16; 3], HardwareError>;
    fn read_battery(&mut self) -> Result<BatteryReading, HardwareError>;
    /// Resets and probes the touch controller. `Ok(false)` means the board
    /// has none; boards without touch keep this default.
    fn initialize_touch(&mut self) -> Result<bool, HardwareError> {
        Ok(false)
    }
    /// Current touch point in the panel's native pixels, or None while
    /// untouched or unreadable.
    fn read_touch(&mut self) -> Option<(u16, u16)> {
        None
    }
    fn show(&mut self, buffer: &[u8], regions: Option<&[Region]>) -> Result<(), HardwareError>;
    fn backlight(&mut self, enabled: bool);
    fn poll(&mut self);
    /// Only true for a positively detected USB host/power signal, not an
    /// assumption based on voltage or the presence of a serial peripheral.
    fn externally_powered(&self) -> bool;
    /// A positively detected debugger; false for UART bridges without DTR sense.
    fn logging_connected(&self) -> bool;
    /// Backend decides whether a receiver is connected; UART may log blindly.
    fn log(&mut self, bytes: &[u8]);
    fn enter_sleep(&mut self) -> Result<(), HardwareError>;
    /// Read both the physical IRQ level and the latched IMU motion status.
    fn wake_status(&mut self) -> Result<WakeStatus, HardwareError>;
    /// Wait for motion or a bounded recovery deadline without busy polling.
    fn wait_for_wake(&mut self);
    /// Disable wake sources, restore active IMU sampling, and wake the LCD,
    /// attempting every step even if an earlier one fails. `Err` means the IMU
    /// or LCD may still be asleep; `Ok(Some(_))` reports a cleanup failure,
    /// such as a wake-on-motion disable timeout, after both were restored.
    fn exit_sleep(&mut self) -> Result<Option<HardwareError>, HardwareError>;
}
