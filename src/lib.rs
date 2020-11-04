//! Adafruid 24 Channel 12-bit PWM Controller
//!
//! Controll the Adafruit 12-bit, 24 channel, TLC 5947 based led controller.
//! https://www.adafruit.com/product/1429
//!
//! The breakout board has 24 pins to drive LEDs and supports common-annode
//! RGB LEDs.  Boards can be chained in series.   The supply voltage is five or
//! more volts with the logic level at either 3 to 5 volts.  I tested it using
//! a Nucleo STM32L432 development board with 3 volt logic, and have used it in
//! projects with Arduinos at 5 volt logic.
//!
//! The protocol is fairly simple.  For each channel, we bit-bang the 12
//! bits to the board.  That will "dim" LEDs attached to that channel.
//!

#![no_std]

use embedded_hal::digital::v2::OutputPin;

pub mod pwm;


/// This is the set of masks we'll use to check if the bit on a 12-bit number is
/// 1 or 0.  Since this is internal to our implementation, we don't need to export
/// it.  Preferred to have the masks in an array for easier iteration.
const PWM_BIT_MASKS: [u16; 12] = [
    0x0800_u16, 0x0400_u16, 0x0200_u16, 0x0100_u16, 0x0080_u16, 0x0040_u16, 0x0020_u16, 0x0010_u16,
    0x0008_u16, 0x0004_u16, 0x0002_u16, 0x0001_u16,
];

/// The role a pin occupies in the device.  The values can be the latch pin,
/// the data pin, the OE pin, or the clock pin.
#[derive(Clone, PartialEq, Debug)]
pub enum PinRole {
    Latch,
    Data,
    OE,
    Clock,
}

/// The error returned from the configured device.  It indicates which pin
/// failed and a message to help debug.
pub struct PinError {
    pub which: PinRole,
    pub message: &'static str,
}

impl PinError {
    fn new(which: &PinRole, message: &'static str) -> Self {
        PinError {
            which: which.clone(),
            message,
        }
    }
}

struct PWMPin<T>
where
    T: OutputPin,
{
    raw_pin: T,
    which_pin: PinRole,
}

impl<T> PWMPin<T>
where
    T: OutputPin,
{
    fn new(raw_pin: T, which_pin: PinRole) -> Self {
        PWMPin { raw_pin, which_pin }
    }
}

impl<T> OutputPin for PWMPin<T>
where
    T: OutputPin,
{
    type Error = PinError;

    fn set_high(&mut self) -> Result<(), Self::Error> {
        match self.raw_pin.set_high() {
            Ok(_) => Ok(()),
            Err(_) => Err(PinError::new(&self.which_pin, "Failed to set high")),
        }
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        match self.raw_pin.set_low() {
            Ok(_) => Ok(()),
            Err(_) => Err(PinError::new(&self.which_pin, "Failed to set low")),
        }
    }
}


/// This represents an individual device.  It has four pins that are used, the
/// L or Latch pin, the D or Data pin, the O or OE pin, and the C or Clock pin.
/// The reason these are generic parameters is that each pin is it's own data
/// struct.  Unless we want to pass references to the OutputPin trait for those pins,
/// the struct needs the generic parameters to allow assigning pins to the device.
///
/// The device has a buffer of 24 integers (16-bit, unsigned) to hold the PWm values.
/// It then has members for the four pins.  We need to expor the struct, but not the
/// individual members.  We don't want someone reaching in and interfering with the
/// protocol.
pub struct PWM5947<L, D, O, C>
where
    L: OutputPin,
    D: OutputPin,
    O: OutputPin,
    C: OutputPin,
{
    buffer: [u16; 24],

    latch: PWMPin<L>,
    data: PWMPin<D>,
    oe: PWMPin<O>,
    clock: PWMPin<C>,
}

impl<L, D, O, C> PWM5947<L, D, O, C>
where
    L: OutputPin,
    D: OutputPin,
    O: OutputPin,
    C: OutputPin,
{
    /// Create a new PWM5947 device.  Passes in the pins that will now be owned
    /// by the device.  
    pub fn new(latch: L, data: D, oe: O, clock: C) -> Self {
        PWM5947 {
            buffer: [0; 24],
            latch: PWMPin::new(latch, PinRole::Latch),
            data: PWMPin::new(data, PinRole::Data),
            oe: PWMPin::new(oe, PinRole::OE),
            clock: PWMPin::new(clock, PinRole::Clock),
        }
    }

    /// During debugging I wanted some way to make sure the device was initialized
    /// to known, good values.  This sets all the pins to "low" and clears the
    /// buffer holding the PWM values to zero.
    pub fn begin(&mut self) -> Result<(), PinError> {
        self.oe.set_low()?;
        self.latch.set_low()?;
        self.data.set_low()?;
        self.clock.set_low()?;

        for i in 0..24 {
            self.buffer[i] = 0x0_u16;
        }

        Ok(())
    }

    /// Writes a value into the given channel.  It basically updates the buffer
    /// of values, making sure the passed in value a 12-bit integer by making it
    /// with the PWM_MASK, above.  We also don't worry about values outsie the
    /// range of 24, but silently.
    pub fn write_pwm(&mut self, channel: &usize, pwm_value: &u16) {
        if *channel < 24 {
            self.buffer[*channel] = *pwm_value & pwm::PWM_MASK;
        }
    }

    /// This sets the buffer back to all zeros and then flushes to turn off all the
    /// LEDs.
    pub fn all_black(&mut self) -> Result<(), PinError> {
        for i in 0..24 {
            self.buffer[i] = 0x0_u16;
        }
        self.flush()
    }

    /// Flushes the values from the buffer to the device.  It starts by making
    /// sure the latch is set to low.  Then, for each channel, it cycles through
    /// the 12 bits in the PWM value.  It toggles the bit by setting the clock low,
    /// the data line high or low, and the sets the clock high.  When it's
    /// finished all 24 channels, it sets the clock log and toggles the latch.
    pub fn flush(&mut self) -> Result<(), PinError> {
        self.latch.set_low()?;

        for i in 0..24 {
            let channel_value: u16 = self.buffer[23 - i];

            for bit_mask in &PWM_BIT_MASKS {
                self.clock.set_low()?;

                if *bit_mask & channel_value != 0 {
                    self.data.set_high()?;
                } else {
                    self.data.set_low()?;
                }

                self.clock.set_high()?;
            }
        }

        self.clock.set_low()?;
        self.latch.set_high()?;
        self.latch.set_low()
    }
}




#[cfg(test)]
mod tests {
    use core::convert::Infallible;
    use embedded_hal::digital::v2::OutputPin;

    /// Fake pin for testing purposes.
    struct FakePin {
        value: bool,
    }

    impl OutputPin for FakePin {
        type Error = Infallible;

        fn set_high(&mut self) -> Result<(), Self::Error> {
            self.value = true;
            Ok(())
        }

        fn set_low(&mut self) -> Result<(), Self::Error> {
            self.value = false;
            Ok(())
        }
    }

    #[test]
    fn test_toggle() {
        let latch = FakePin { value: false };
        let oe = FakePin { value: false };
        let data = FakePin { value: false };
        let clock = FakePin { value: false };

        let mut device = crate::PWM5947::new(latch, data, oe, clock);
        let res = device.begin();
        assert!(res.is_ok());

        for i in 0..24 {
            let val: u16 = i as u16;
            device.write_pwm(&i, &val);
        }

        for i in 0..24 {
            assert_eq!(device.buffer[i], i as u16);
        }
    }

    #[test]
    fn test_begin() {
        let latch = FakePin { value: true };
        let oe = FakePin { value: true };
        let data = FakePin { value: true };
        let clock = FakePin { value: true };

        let mut device = crate::PWM5947::new(latch, data, oe, clock);
        for i in 0..24 {
            device.buffer[i] = 0x10;
        }

        let res = device.begin();
        assert!(res.is_ok());

        for i in 0..24 {
            assert_eq!(device.buffer[i], 0);
        }

        assert!(!device.latch.raw_pin.value);
        assert!(!device.clock.raw_pin.value);
        assert!(!device.oe.raw_pin.value);
        assert!(!device.data.raw_pin.value);
    }

    struct FailingPin {
        will_fail: bool,
        value: bool,
    }

    impl FailingPin {
        fn new(will_fail: &bool, value: &bool) -> Self {
            FailingPin {
                will_fail: *will_fail,
                value: *value,
            }
        }
    }

    impl OutputPin for FailingPin {
        type Error = &'static str;

        fn set_high(&mut self) -> Result<(), Self::Error> {
            if self.will_fail {
                Err("Failed")
            } else {
                self.value = true;
                Ok(())
            }
        }

        fn set_low(&mut self) -> Result<(), Self::Error> {
            if self.will_fail {
                Err("Failed")
            } else {
                self.value = false;
                Ok(())
            }
        }
    }

    #[test]
    fn test_failing_pin() {
        let latch = FakePin { value: true };
        let oe = FailingPin::new(&true, &true);
        let data = FakePin { value: true };
        let clock = FakePin { value: true };

        let mut device = crate::PWM5947::new(latch, data, oe, clock);
        let res = device.begin();
        if let Err(e) = res {
            assert_eq!(e.which, crate::PinRole::OE);
        } else {
            assert!(false);
        }
    }

}
