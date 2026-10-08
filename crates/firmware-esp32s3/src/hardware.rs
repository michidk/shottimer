//! Fixed onboard wiring. Alternative assignments require physical rewiring.
macro_rules! select_hardware {
    ($p:ident) => {
        (
            $p.GPIO8,  // LCD DC
            $p.GPIO9,  // LCD CS
            $p.GPIO10, // LCD CLK
            $p.GPIO11, // LCD MOSI
            $p.GPIO14, // LCD reset
            $p.GPIO2,  // LCD backlight
            $p.GPIO1,  // Battery ADC1
            $p.GPIO6,  // IMU SDA
            $p.GPIO7,  // IMU SCL
            $p.GPIO3,  // IMU INT2
            $p.GPIO13, // Touch reset
            $p.GPIO43, // CH343 UART TX
            $p.SPI2, $p.I2C0, $p.UART0,
        )
    };
}
pub(crate) use select_hardware;
