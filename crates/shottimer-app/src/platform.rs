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
pub enum HardwareError {
    Imu,
    Display,
    Battery,
    Sleep,
}

pub trait Platform {
    /// Monotonic uptime in milliseconds, including light-sleep intervals.
    fn now_ms(&self) -> u64;
    fn delay_ms(&mut self, ms: u32);
    fn initialize_imu(&mut self) -> Result<u8, HardwareError>;
    /// Signed QMI8658 counts in ±8 g mode, not register bytes.
    fn read_accel_raw(&mut self) -> Result<[i16; 3], HardwareError>;
    fn read_battery(&mut self) -> Result<BatteryReading, HardwareError>;
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
    fn motion_pending(&mut self) -> Result<bool, HardwareError>;
    /// Wait for motion or a bounded recovery deadline without busy polling.
    fn wait_for_wake(&mut self);
    fn exit_sleep(&mut self) -> Result<(), HardwareError>;
}
