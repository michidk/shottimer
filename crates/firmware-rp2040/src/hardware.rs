//! Compile-time GPIO and peripheral mapping for the Waveshare board.
//!
//! Edit the `gpN`, `SPI0`/`SPI1`, and `I2C0`/`I2C1` selections below.
//! Keeping the HAL's pin types lets the compiler reject reused pins and
//! unsupported SPI, I2C, or ADC mappings. Built-in board wiring is fixed;
//! alternate assignments require matching physical wiring.

macro_rules! select_hardware {
    ($pins:ident, $pac:ident) => {
        (
            $pins.gp8,  // LCD data/command
            $pins.gp9,  // LCD chip select (software controlled)
            $pins.gp10, // LCD SPI clock
            $pins.gp11, // LCD SPI MOSI
            $pins.gp12, // LCD reset
            $pins.gp25, // LCD backlight
            $pins.gp29, // Battery ADC (GP26..GP29)
            $pins.gp6,  // IMU I2C SDA
            $pins.gp7,  // IMU I2C SCL
            $pins.gp24, // IMU INT2 wake-on-motion
            $pac.SPI1,  // LCD SPI peripheral
            $pac.I2C1,  // IMU I2C peripheral
        )
    };
}

pub(crate) use select_hardware;

// GP25 uses PWM slice 4, channel B. Update these together with the backlight
// GPIO above when adapting the wiring; output_to() checks the mapping.
macro_rules! select_backlight_pwm {
    ($slices:ident) => {{
        let slice = &mut $slices.pwm4;
        slice.set_top(999);
        slice.set_div_int(8);
        slice.enable();
        &mut slice.channel_b
    }};
}

pub(crate) use select_backlight_pwm;
