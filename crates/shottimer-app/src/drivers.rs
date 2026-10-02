//! Reusable display and IMU drivers; no MCU-specific dependencies.
use crate::{
    HardwareError, Region,
    display_bus::{self, Shared},
};
use core::cell::RefCell;
use embassy_futures::block_on;
use embedded_hal::{delay::DelayNs, digital::OutputPin, spi::SpiBus};
use embedded_hal_async::i2c::I2c;
use gc9a01a_driver::{GC9A01A, Orientation};
use ph_qmi8658::{
    AccelConfig, AccelOutputDataRate, AccelRange, Config, I2cConfig, InterruptConfig, InterruptPin,
    Qmi8658Address, Qmi8658I2c, WomConfig, WomInterruptLevel,
};
use shottimer_core::settings::{DISPLAY_ROTATION_DEGREES, SLEEP_WAKE_THRESHOLD_MG};

pub struct Imu<I: I2c> {
    inner: Qmi8658I2c<I>,
}

impl<I: I2c> Imu<I> {
    pub fn new(bus: I) -> Self {
        let config = active_config();
        // QMI8658 on both boards uses little-endian samples.
        let interface = I2cConfig::new(Qmi8658Address::Primary.addr()).with_big_endian(false);
        Self {
            inner: Qmi8658I2c::with_i2c_config(bus, None, None, config, interface),
        }
    }
    pub fn initialize<D: DelayNs>(&mut self, delay: &mut D) -> Result<u8, HardwareError> {
        let address = block_on(self.inner.init_with_addresses(
            &mut SyncDelay(delay),
            &[
                Qmi8658Address::Primary.addr(),
                Qmi8658Address::Secondary.addr(),
            ],
        ))
        .map_err(|_| HardwareError::Imu)?;
        // Route CTRL9 completion to STATUSINT bit 7. The driver handles the
        // completion/ACK handshake without reading STATUS1 motion latches.
        block_on(
            self.inner.apply_interrupt_config(
                InterruptConfig::new()
                    .with_ctrl9_handshake_statusint(true)
                    .with_motion_pin(InterruptPin::Int2),
            ),
        )
        .map_err(|_| HardwareError::Imu)?;
        Ok(address)
    }
    pub fn read_raw(&mut self) -> Result<[i16; 3], HardwareError> {
        block_on(self.inner.read_accel_raw())
            .map(|v| [v.data.x, v.data.y, v.data.z])
            .map_err(|_| HardwareError::Imu)
    }
    pub fn enter_sleep<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), HardwareError> {
        self.inner.set_config(
            Config::new()
                .with_accel_config(AccelConfig::new(
                    AccelRange::G8,
                    AccelOutputDataRate::LowPowerHz128,
                ))
                .without_gyro(),
        );
        // enable_wom applies the low-power configuration with sensors disabled,
        // then enables acceleration only after the command has completed.
        let wom = WomConfig::new(SLEEP_WAKE_THRESHOLD_MG)
            .with_blanking_samples(4)
            .with_interrupt(InterruptPin::Int2, WomInterruptLevel::Low);
        block_on(self.inner.enable_wom(&mut SyncDelay(delay), wom)).map_err(command_error)
    }
    /// Restores active sampling. `Ok(Some(_))` reports a failed WoM disable
    /// that did not prevent sampling from resuming.
    pub fn exit_sleep<D: DelayNs>(
        &mut self,
        delay: &mut D,
    ) -> Result<Option<HardwareError>, HardwareError> {
        let disable_result = block_on(self.inner.disable_wom(&mut SyncDelay(delay)));
        // Restore normal sampling even if disabling WoM times out during recovery.
        self.inner.set_config(active_config());
        block_on(self.inner.apply_config()).map_err(|_| HardwareError::Imu)?;
        delay.delay_ms(20);
        Ok(disable_result.err().map(command_error))
    }
    pub fn motion_pending(&mut self) -> Result<bool, HardwareError> {
        block_on(self.inner.read_interrupt_status())
            .map(|s| s.wake_on_motion)
            .map_err(|_| HardwareError::Imu)
    }
}

fn command_error(error: ph_qmi8658::Error) -> HardwareError {
    match error {
        ph_qmi8658::Error::NotReady => HardwareError::ImuCommandTimeout,
        _ => HardwareError::Imu,
    }
}

fn active_config() -> Config {
    Config::new()
        .with_accel_config(AccelConfig::new(
            AccelRange::G8,
            AccelOutputDataRate::Hz1000,
        ))
        .without_gyro()
}

pub struct SyncDelay<'a, D>(pub &'a mut D);
impl<D: DelayNs> embedded_hal_async::delay::DelayNs for SyncDelay<'_, D> {
    async fn delay_ns(&mut self, ns: u32) {
        self.0.delay_ns(ns);
    }
}

pub struct Display<'a, S: SpiBus<u8>, D: OutputPin, C: OutputPin, R: OutputPin> {
    driver: GC9A01A<Shared<'a, S>, Shared<'a, D>, Shared<'a, C>, R>,
    spi: &'a RefCell<S>,
    dc: &'a RefCell<D>,
    cs: &'a RefCell<C>,
}

impl<'a, S: SpiBus<u8>, D: OutputPin, C: OutputPin, R: OutputPin> Display<'a, S, D, C, R> {
    /// Performs no bus I/O; call `initialize` before showing frames.
    pub fn new(spi: &'a RefCell<S>, dc: &'a RefCell<D>, cs: &'a RefCell<C>, reset: R) -> Self {
        Self {
            driver: GC9A01A::new(Shared(spi), Shared(dc), Shared(cs), reset, false, 240, 240),
            spi,
            dc,
            cs,
        }
    }
    /// Hardware-resets and configures the panel; safe to repeat after faults.
    pub fn initialize<T: DelayNs>(&mut self, delay: &mut T) -> Result<(), HardwareError> {
        self.driver
            .init(delay)
            .map_err(|_| HardwareError::Display)?;
        let orientation = match DISPLAY_ROTATION_DEGREES {
            0 => Orientation::Portrait,
            90 => Orientation::Landscape,
            180 => Orientation::PortraitSwapped,
            270 => Orientation::LandscapeSwapped,
            _ => unreachable!(),
        };
        self.driver
            .set_orientation(&orientation)
            .map_err(|_| HardwareError::Display)
    }
    pub fn show(&mut self, buffer: &[u8], regions: Option<&[Region]>) -> Result<(), HardwareError> {
        if let Some(regions) = regions {
            for &(x, y, w, h) in regions {
                self.driver
                    .show_region(buffer, x, y, w, h)
                    .map_err(|_| HardwareError::Display)?;
            }
        } else {
            self.driver
                .show(buffer)
                .map_err(|_| HardwareError::Display)?;
        }
        Ok(())
    }
    pub fn sleep<T: DelayNs>(&mut self, delay: &mut T) -> Result<(), HardwareError> {
        self.command(0x28)?;
        self.command(0x10)?;
        delay.delay_ms(120);
        Ok(())
    }
    pub fn wake<T: DelayNs>(&mut self, delay: &mut T) -> Result<(), HardwareError> {
        self.command(0x11)?;
        delay.delay_ms(120);
        self.command(0x29)
    }
    fn command(&mut self, command: u8) -> Result<(), HardwareError> {
        display_bus::command(self.spi, self.dc, self.cs, command)
            .map_err(|_| HardwareError::Display)
    }
}

/// Shared `Platform::exit_sleep` sequence: both peripherals are restored even
/// if one fails, so an IMU error cannot leave the LCD asleep.
pub fn restore_after_sleep<I, S, D, C, R, T>(
    imu: &mut Imu<I>,
    display: &mut Display<'_, S, D, C, R>,
    delay: &mut T,
) -> Result<Option<HardwareError>, HardwareError>
where
    I: I2c,
    S: SpiBus<u8>,
    D: OutputPin,
    C: OutputPin,
    R: OutputPin,
    T: DelayNs,
{
    let imu_result = imu.exit_sleep(delay);
    let display_result = display.wake(delay);
    let cleanup = imu_result?;
    display_result?;
    Ok(cleanup)
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use embedded_hal_async::i2c::Operation;
    use std::{cell::RefCell, rc::Rc, vec::Vec};

    struct Sensor {
        registers: [u8; 128],
        writes: Vec<(u8, u8)>,
        command_pending: bool,
        acknowledge: bool,
        offline: bool,
    }

    impl Sensor {
        fn new() -> Self {
            let mut registers = [0; 128];
            registers[0] = 5;
            Self {
                registers,
                writes: Vec::new(),
                command_pending: false,
                acknowledge: true,
                offline: false,
            }
        }
    }

    struct Bus(Rc<RefCell<Sensor>>);
    impl embedded_hal::i2c::ErrorType for Bus {
        type Error = embedded_hal::i2c::ErrorKind;
    }

    impl I2c for Bus {
        async fn transaction(
            &mut self,
            _: u8,
            operations: &mut [Operation<'_>],
        ) -> Result<(), Self::Error> {
            let mut sensor = self.0.borrow_mut();
            if sensor.offline {
                return Err(embedded_hal::i2c::ErrorKind::Other);
            }
            let mut register = 0;
            for operation in operations {
                match operation {
                    Operation::Write(bytes) => {
                        register = bytes[0];
                        for &value in &bytes[1..] {
                            sensor.registers[register as usize] = value;
                            sensor.writes.push((register, value));
                            if register == 0x0a && value == 8 {
                                sensor.command_pending = true;
                            }
                            if register == 0x0a && value == 0 {
                                sensor.command_pending = false;
                            }
                            register += 1;
                        }
                    }
                    Operation::Read(bytes) => {
                        for value in bytes.iter_mut() {
                            // Commands must use STATUSINT directly, not STATUS1
                            // (which also clears motion/interrupt latches).
                            assert!(!(sensor.command_pending && register == 0x2f));
                            *value =
                                if register == 0x2d && sensor.command_pending && sensor.acknowledge
                                {
                                    assert_eq!(sensor.registers[0x09] & 0x80, 0x80);
                                    0x80
                                } else {
                                    sensor.registers[register as usize]
                                };
                            register += 1;
                        }
                    }
                }
            }
            Ok(())
        }
    }

    struct Delay;
    impl DelayNs for Delay {
        fn delay_ns(&mut self, _: u32) {}
    }

    #[test]
    fn repeated_sleep_wake_uses_statusint_handshake_and_correct_sensor_order() {
        let sensor = Rc::new(RefCell::new(Sensor::new()));
        let mut imu = Imu::new(Bus(sensor.clone()));
        let mut delay = Delay;
        imu.initialize(&mut delay).unwrap();
        assert_eq!(sensor.borrow().registers[0x09], 0x80);
        for _ in 0..3 {
            let start = sensor.borrow().writes.len();
            imu.enter_sleep(&mut delay).unwrap();
            {
                let state = sensor.borrow();
                let writes = &state.writes[start..];
                assert_eq!(writes[0], (0x08, 0));
                assert_eq!(writes.last(), Some(&(0x08, 1)));
                assert!(writes.contains(&(0x0a, 8)));
                assert_eq!(state.registers[0x03] & 0x0f, 0x0c);
                assert_eq!(state.registers[0x0b], SLEEP_WAKE_THRESHOLD_MG);
                assert_eq!(state.registers[0x0c], 0x44);
                assert!(!state.command_pending);
            }
            imu.exit_sleep(&mut delay).unwrap();
            assert_eq!(sensor.borrow().registers[0x03] & 0x0f, 3);
            assert_eq!(sensor.borrow().registers[0x08], 1);
        }
    }

    #[test]
    fn command_timeout_is_reported_and_recovery_restores_active_sampling() {
        let sensor = Rc::new(RefCell::new(Sensor::new()));
        let mut imu = Imu::new(Bus(sensor.clone()));
        let mut delay = Delay;
        imu.initialize(&mut delay).unwrap();
        sensor.borrow_mut().acknowledge = false;
        assert_eq!(
            imu.enter_sleep(&mut delay),
            Err(HardwareError::ImuCommandTimeout)
        );
        assert_eq!(sensor.borrow().registers[0x08], 0);
        assert_eq!(
            imu.exit_sleep(&mut delay),
            Ok(Some(HardwareError::ImuCommandTimeout))
        );
        assert_eq!(sensor.borrow().registers[0x03] & 0x0f, 3);
        assert_eq!(sensor.borrow().registers[0x08], 1);
    }

    #[derive(Default)]
    struct Panel {
        /// Bytes written while D/C selected command mode.
        commands: Vec<u8>,
        command_mode: bool,
        fail: bool,
    }
    struct PanelBus(Rc<RefCell<Panel>>);
    struct PanelDc(Rc<RefCell<Panel>>);
    struct Pin;
    impl embedded_hal::spi::ErrorType for PanelBus {
        type Error = embedded_hal::spi::ErrorKind;
    }
    impl SpiBus<u8> for PanelBus {
        fn read(&mut self, _: &mut [u8]) -> Result<(), Self::Error> {
            unreachable!()
        }
        fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
            let mut panel = self.0.borrow_mut();
            if panel.fail {
                return Err(embedded_hal::spi::ErrorKind::Other);
            }
            if panel.command_mode {
                panel.commands.extend_from_slice(bytes);
            }
            Ok(())
        }
        fn transfer(&mut self, _: &mut [u8], _: &[u8]) -> Result<(), Self::Error> {
            unreachable!()
        }
        fn transfer_in_place(&mut self, _: &mut [u8]) -> Result<(), Self::Error> {
            unreachable!()
        }
        fn flush(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
    }
    impl embedded_hal::digital::ErrorType for PanelDc {
        type Error = core::convert::Infallible;
    }
    impl OutputPin for PanelDc {
        fn set_low(&mut self) -> Result<(), Self::Error> {
            self.0.borrow_mut().command_mode = true;
            Ok(())
        }
        fn set_high(&mut self) -> Result<(), Self::Error> {
            self.0.borrow_mut().command_mode = false;
            Ok(())
        }
    }
    impl embedded_hal::digital::ErrorType for Pin {
        type Error = core::convert::Infallible;
    }
    impl OutputPin for Pin {
        fn set_low(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn set_high(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[test]
    fn display_initialization_is_deferred_and_repeatable() {
        let panel = Rc::new(RefCell::new(Panel::default()));
        let spi = RefCell::new(PanelBus(panel.clone()));
        let dc = RefCell::new(PanelDc(panel.clone()));
        let cs = RefCell::new(Pin);
        let mut display = Display::new(&spi, &dc, &cs, Pin);
        assert!(panel.borrow().commands.is_empty());
        panel.borrow_mut().fail = true;
        assert_eq!(display.initialize(&mut Delay), Err(HardwareError::Display));
        panel.borrow_mut().fail = false;
        display.initialize(&mut Delay).unwrap();
        let first = panel.borrow().commands.len();
        assert!(first > 0);
        display.initialize(&mut Delay).unwrap();
        assert_eq!(panel.borrow().commands.len(), first * 2);
    }

    #[test]
    fn sleep_restore_wakes_lcd_when_imu_cleanup_or_restore_fails() {
        let panel = Rc::new(RefCell::new(Panel::default()));
        let spi = RefCell::new(PanelBus(panel.clone()));
        let dc = RefCell::new(PanelDc(panel.clone()));
        let cs = RefCell::new(Pin);
        let mut display = Display::new(&spi, &dc, &cs, Pin);
        let sensor = Rc::new(RefCell::new(Sensor::new()));
        let mut imu = Imu::new(Bus(sensor.clone()));
        let mut delay = Delay;
        imu.initialize(&mut delay).unwrap();

        // A WoM-disable timeout is cleanup only: sampling and the LCD resume.
        sensor.borrow_mut().acknowledge = false;
        imu.enter_sleep(&mut delay).ok();
        assert_eq!(
            restore_after_sleep(&mut imu, &mut display, &mut delay),
            Ok(Some(HardwareError::ImuCommandTimeout))
        );
        assert_eq!(panel.borrow().commands, [0x11, 0x29]);
        assert_eq!(sensor.borrow().registers[0x08], 1);

        // An unreachable IMU must not stop the LCD from waking.
        panel.borrow_mut().commands.clear();
        sensor.borrow_mut().offline = true;
        assert_eq!(
            restore_after_sleep(&mut imu, &mut display, &mut delay),
            Err(HardwareError::Imu)
        );
        assert_eq!(panel.borrow().commands, [0x11, 0x29]);
        sensor.borrow_mut().offline = false;

        // An LCD failure is reported even though the IMU was restored first.
        panel.borrow_mut().commands.clear();
        panel.borrow_mut().fail = true;
        sensor.borrow_mut().acknowledge = true;
        assert_eq!(
            restore_after_sleep(&mut imu, &mut display, &mut delay),
            Err(HardwareError::Display)
        );
        assert_eq!(sensor.borrow().registers[0x08], 1);
    }
}
