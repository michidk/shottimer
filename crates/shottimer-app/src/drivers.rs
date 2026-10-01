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
    AccelConfig, AccelOutputDataRate, AccelRange, Config, I2cConfig, InterruptPin, Qmi8658Address,
    Qmi8658I2c, WomConfig, WomInterruptLevel,
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
        block_on(self.inner.init_with_addresses(
            &mut SyncDelay(delay),
            &[
                Qmi8658Address::Primary.addr(),
                Qmi8658Address::Secondary.addr(),
            ],
        ))
        .map_err(|_| HardwareError::Imu)
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
        block_on(self.inner.apply_config()).map_err(|_| HardwareError::Imu)?;
        let wom = WomConfig::new(SLEEP_WAKE_THRESHOLD_MG)
            .with_blanking_samples(4)
            .with_interrupt(InterruptPin::Int2, WomInterruptLevel::Low);
        block_on(self.inner.enable_wom(&mut SyncDelay(delay), wom)).map_err(|_| HardwareError::Imu)
    }
    pub fn exit_sleep<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), HardwareError> {
        block_on(self.inner.disable_wom(&mut SyncDelay(delay))).map_err(|_| HardwareError::Imu)?;
        self.inner.set_config(active_config());
        block_on(self.inner.apply_config()).map_err(|_| HardwareError::Imu)?;
        delay.delay_ms(20);
        Ok(())
    }
    pub fn motion_pending(&mut self) -> Result<bool, HardwareError> {
        block_on(self.inner.read_interrupt_status())
            .map(|s| s.wake_on_motion)
            .map_err(|_| HardwareError::Imu)
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
    pub fn new<T: DelayNs>(
        spi: &'a RefCell<S>,
        dc: &'a RefCell<D>,
        cs: &'a RefCell<C>,
        reset: R,
        delay: &mut T,
    ) -> Result<Self, HardwareError> {
        let mut driver = GC9A01A::new(Shared(spi), Shared(dc), Shared(cs), reset, false, 240, 240);
        driver.init(delay).map_err(|_| HardwareError::Display)?;
        let orientation = match DISPLAY_ROTATION_DEGREES {
            0 => Orientation::Portrait,
            90 => Orientation::Landscape,
            180 => Orientation::PortraitSwapped,
            270 => Orientation::LandscapeSwapped,
            _ => unreachable!(),
        };
        driver
            .set_orientation(&orientation)
            .map_err(|_| HardwareError::Display)?;
        Ok(Self {
            driver,
            spi,
            dc,
            cs,
        })
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
