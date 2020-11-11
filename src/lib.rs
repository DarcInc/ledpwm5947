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

/// Channel identifies a legal channel on the board.  There are only 24
/// legal values for channel.  These constants represent the 24 channels.
/// It may be necessary to switch to a non-public channel constructor so
/// only these 24 channels can be instantiated, and the channel number is
/// opaque.
pub struct Channel(usize);
pub const C1: Channel = Channel(0);
pub const C2: Channel = Channel(1);
pub const C3: Channel = Channel(2);
pub const C4: Channel = Channel(3);
pub const C5: Channel = Channel(4);
pub const C6: Channel = Channel(5);
pub const C7: Channel = Channel(6);
pub const C8: Channel = Channel(7);
pub const C9: Channel = Channel(8);
pub const C10: Channel = Channel(9);
pub const C11: Channel = Channel(10);
pub const C12: Channel = Channel(11);
pub const C13: Channel = Channel(12);
pub const C14: Channel = Channel(13);
pub const C15: Channel = Channel(14);
pub const C16: Channel = Channel(15);
pub const C17: Channel = Channel(16);
pub const C18: Channel = Channel(17);
pub const C19: Channel = Channel(18);
pub const C20: Channel = Channel(19);
pub const C21: Channel = Channel(20);
pub const C22: Channel = Channel(21);
pub const C23: Channel = Channel(22);
pub const C24: Channel = Channel(23);

/// A slice of all channels to facilitate logic that iterates over the list of
/// available channels.
pub const ALL_CHANNELS: &[Channel] = &[
    C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C14, C15, C16, C17, C18, C19, C20, C21,
    C22, C23, C24,
];

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
    buffer: [pwm::PWMValue; 24],

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
            buffer: [pwm::PWMValue::min(); 24],
            latch: PWMPin::new(latch, PinRole::Latch),
            data: PWMPin::new(data, PinRole::Data),
            oe: PWMPin::new(oe, PinRole::OE),
            clock: PWMPin::new(clock, PinRole::Clock),
        }
    }

    /// During debugging I wanted some way to make sure the device was initialized
    /// to known, good values.  It clears the data in the buffer and sets it to the
    /// PWM's `min` value.
    pub fn begin(&mut self) -> Result<(), PinError> {
        self.oe.set_low()?;
        self.latch.set_low()?;
        self.data.set_low()?;
        self.clock.set_low()?;

        for i in 0..24 {
            self.buffer[i] = pwm::PWMValue::min();
        }

        Ok(())
    }

    /// Writes a value into the given channel.  It saves the PWM value into the 
    /// buffer for the given channel.
    pub fn write_pwm(&mut self, channel: &Channel, pwm_value: &pwm::PWMValue) {
        self.buffer[channel.0] = *pwm_value;
    }

    /// This sets the buffer back to all zeros and then flushes to turn off all the
    /// LEDs.
    pub fn all_black(&mut self) -> Result<(), PinError> {
        for channel in ALL_CHANNELS {
            self.buffer[channel.0] = pwm::PWMValue::min();
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

        for channel in ALL_CHANNELS.iter().rev() {
            let channel_value = self.buffer[channel.0];

            let bit_values = channel_value.bits();

            for i in 0..bit_values.len() {
                self.clock.set_low()?;

                if bit_values[i] {
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

    // Fake pin for testing purposes.
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

    use crate::pwm::PWMValue;

    #[test]
    fn test_toggle() {
        let latch = FakePin { value: false };
        let oe = FakePin { value: false };
        let data = FakePin { value: false };
        let clock = FakePin { value: false };

        let mut device = crate::PWM5947::new(latch, data, oe, clock);
        let res = device.begin();
        assert!(res.is_ok());

        for channel in crate::ALL_CHANNELS {
            let val = PWMValue::new(channel.0 as i32);
            device.write_pwm(channel, &val);
        }

        for i in 0..24 {
            assert_eq!(device.buffer[i], PWMValue::new(i as i32));
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
            device.buffer[i] = PWMValue::new(0x10);
        }

        let res = device.begin();
        assert!(res.is_ok());

        for i in 0..24 {
            assert_eq!(device.buffer[i], PWMValue::min());
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
