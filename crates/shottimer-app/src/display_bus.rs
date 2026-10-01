//! Share the existing display transport with its small sleep-command interface.
use core::cell::RefCell;
use embedded_hal::{digital, spi};

pub struct Shared<'a, T>(pub &'a RefCell<T>);

impl<T: digital::ErrorType> digital::ErrorType for Shared<'_, T> {
    type Error = T::Error;
}
impl<T: digital::OutputPin> digital::OutputPin for Shared<'_, T> {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.0.borrow_mut().set_low()
    }
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.0.borrow_mut().set_high()
    }
}
impl<T: spi::ErrorType> spi::ErrorType for Shared<'_, T> {
    type Error = T::Error;
}
impl<T: spi::SpiBus<u8>> spi::SpiBus<u8> for Shared<'_, T> {
    fn read(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        self.0.borrow_mut().read(words)
    }
    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        self.0.borrow_mut().write(words)
    }
    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        self.0.borrow_mut().transfer(read, write)
    }
    fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        self.0.borrow_mut().transfer_in_place(words)
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.0.borrow_mut().flush()
    }
}

#[allow(clippy::result_unit_err)]
pub fn command<S: spi::SpiBus<u8>, D: digital::OutputPin, C: digital::OutputPin>(
    spi: &RefCell<S>,
    dc: &RefCell<D>,
    cs: &RefCell<C>,
    command: u8,
) -> Result<(), ()> {
    dc.borrow_mut().set_low().map_err(|_| ())?;
    cs.borrow_mut().set_low().map_err(|_| ())?;
    let result = {
        let mut bus = spi.borrow_mut();
        bus.write(&[command]).and_then(|_| bus.flush())
    };
    let deselect = cs.borrow_mut().set_high();
    result.map_err(|_| ())?;
    deselect.map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::convert::Infallible;

    #[derive(Default)]
    struct Pin(bool);
    impl digital::ErrorType for Pin {
        type Error = Infallible;
    }
    impl digital::OutputPin for Pin {
        fn set_low(&mut self) -> Result<(), Self::Error> {
            self.0 = false;
            Ok(())
        }
        fn set_high(&mut self) -> Result<(), Self::Error> {
            self.0 = true;
            Ok(())
        }
    }
    #[derive(Default)]
    struct Bus {
        command: u8,
        flushed: bool,
        fail: bool,
    }
    impl spi::ErrorType for Bus {
        type Error = spi::ErrorKind;
    }
    impl spi::SpiBus<u8> for Bus {
        fn read(&mut self, _: &mut [u8]) -> Result<(), Self::Error> {
            unreachable!()
        }
        fn transfer(&mut self, _: &mut [u8], _: &[u8]) -> Result<(), Self::Error> {
            unreachable!()
        }
        fn transfer_in_place(&mut self, _: &mut [u8]) -> Result<(), Self::Error> {
            unreachable!()
        }
        fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
            if self.fail {
                return Err(spi::ErrorKind::Other);
            }
            self.command = bytes[0];
            Ok(())
        }
        fn flush(&mut self) -> Result<(), Self::Error> {
            self.flushed = true;
            Ok(())
        }
    }
    #[test]
    fn sleep_commands_flush_and_release_chip_select() {
        let bus = RefCell::new(Bus::default());
        let dc = RefCell::new(Pin(true));
        let cs = RefCell::new(Pin(true));
        for value in [0x28, 0x10, 0x11, 0x29] {
            assert_eq!(command(&bus, &dc, &cs, value), Ok(()));
            assert_eq!(bus.borrow().command, value);
            assert!(bus.borrow().flushed);
            assert!(!dc.borrow().0);
            assert!(cs.borrow().0);
        }
    }
    #[test]
    fn failed_command_still_releases_chip_select() {
        let bus = RefCell::new(Bus {
            fail: true,
            ..Bus::default()
        });
        let dc = RefCell::new(Pin(true));
        let cs = RefCell::new(Pin(true));
        assert_eq!(command(&bus, &dc, &cs, 0x10), Err(()));
        assert!(cs.borrow().0);
    }
    #[test]
    fn shared_transport_remains_usable_after_sleep_command() {
        let bus = RefCell::new(Bus::default());
        let dc = RefCell::new(Pin(true));
        let cs = RefCell::new(Pin(true));
        command(&bus, &dc, &cs, 0x11).unwrap();
        spi::SpiBus::write(&mut Shared(&bus), &[0x29]).unwrap();
        digital::OutputPin::set_high(&mut Shared(&dc)).unwrap();
        assert_eq!(bus.borrow().command, 0x29);
        assert!(dc.borrow().0);
    }
}
