//! The PWM module encapsulates the PWM values passed to the 5947.  This shouldn't
//! be confused with the PWM from the hardware hal or processor specific hal.
//!
//! PWM values are opaque types that represent a duty cycle for a driven PWM.
//! The are changed by stepping them up and down, using a Step.  it is possible
//! that the stepped value exceeds the PWM limits (usually 0 and some 8, 10,
//! or 12 bit value).  In this case an `Overflow` or `Underflow` error is
//! returned.
//!
//! New PWM values are clamped between min and max.  Step values are clamped
//! between -min and max.  Creating a new PWM value does not cause an error,
//! but operations are intended to highlight code issues that might indifinitely
//! loop on stepping up or down a PWM, thinking the limit has not been reached.

/// The PWM_MASK allows us to "mask" the extra bits in a 16 bit integer, which
/// is what we're using to store the PWM state.  Since this is used internally,
/// to clamp values to a valid 12-bit number, we don't need to export it.
pub const PWM_MASK: u16 = 0x0fff;

/// The PWM value is a number between 0 and the maximum 12-bit value.  As an
/// invariant, the PWM value can never be below 0 or above 4095.
#[derive(Copy, Clone, PartialOrd, PartialEq, Debug)]
pub struct PWMValue {
    raw: i16,
}

/// The range error is returned when the math around PWM values either falls
/// below zero or above the max 12-bit value.  It also applies to steps, where
/// the resulting step is below -4095 or above 4095.
#[derive(PartialOrd, PartialEq, Debug)]
pub enum RangeError {
    Underflow,
    Overflow,
}

/// A step is a fixed amount that can be added to a PWM value to change its value.
/// The specific invariant is that the step can never be less than -4095 or above
/// 4095.
#[derive(Copy, Clone, PartialOrd, PartialEq, Debug)]
pub struct Step {
    amount: i16,
}

/// This is the set of masks we'll use to check if the bit on a 12-bit number is
/// 1 or 0.  Since this is internal to our implementation, we don't need to export
/// it.  Preferred to have the masks in an array for easier iteration.
const PWM_BIT_MASKS: [u16; 12] = [
    0x0800_u16, 0x0400_u16, 0x0200_u16, 0x0100_u16, 0x0080_u16, 0x0040_u16, 0x0020_u16, 0x0010_u16,
    0x0008_u16, 0x0004_u16, 0x0002_u16, 0x0001_u16,
];

impl Step {
    /// Create a new Step from a raw numeric value.  The step will be clamped to
    /// the range `-PWM_MASK` .. `PWM_MASK`.  There are no preconditions, but the
    /// post-condition to creating the step is it falls in the valid range.
    ///
    /// ```
    /// use ledpwm5947::pwm::Step;
    ///
    /// let too_large_step = Step::new(5000);
    /// let max_step = Step::new(4095);
    ///
    /// assert_eq!(too_large_step, max_step);
    ///
    /// let too_large_step = Step::new(-5000);
    /// let max_step = Step::new(-4095);
    ///
    /// assert_eq!(too_large_step, max_step);
    /// ```
    pub fn new(amount: i32) -> Self {
        if amount > PWM_MASK as i32 {
            Step {
                amount: PWM_MASK as i16,
            }
        } else if amount < -(PWM_MASK as i32) {
            Step {
                amount: -(PWM_MASK as i16),
            }
        } else {
            Step {
                amount: amount as i16,
            }
        }
    }

    /// Reverse the direction of a step.  There are no preconditions and the
    /// post-condition is that the step is the same magnitude but opposite sign.
    ///
    /// ```
    /// use ledpwm5947::pwm::Step;
    ///
    /// let forward_step = Step::new(10);
    /// let reversed = forward_step.reverse();
    ///
    /// assert_eq!(Step::new(-10), reversed);
    /// ```
    pub fn reverse(&self) -> Self {
        Step {
            amount: -self.amount,
        }
    }

    /// The checked_new function returns a range error if the new Step value is
    /// out of range.  This is useful where an error is desirable if the logic
    /// can produce and invalid step.  There are no preconditions.  The post-
    /// conditions are that the returned step falls in the range or an error
    /// is returned indicating `underflow` if the value is less than -4095 or
    /// `overflow` if the value is above 4095.
    ///
    /// ```
    /// use ledpwm5947::pwm::Step;
    /// use ledpwm5947::pwm::RangeError;
    ///
    /// let normal_step = Step::checked_new(100).expect("The value indicates a valid step");
    /// if let Err(v) = Step::checked_new(5000) {
    ///     assert_eq!(v, RangeError::Overflow);
    /// } else {
    ///     assert!(false, "Should have returned an error");
    /// }
    ///
    /// if let Err(v) = Step::checked_new(-5000) {
    ///     assert_eq!(v, RangeError::Underflow)
    /// } else {
    ///     assert!(false, "Should have returned an error");
    /// }
    /// ```
    pub fn checked_new(amount: i16) -> Result<Self, RangeError> {
        if amount < -4095 {
            Err(RangeError::Underflow)
        } else if amount > 4095 as i16 {
            Err(RangeError::Overflow)
        } else {
            Ok(Step { amount })
        }
    }

    /// Doubles the step magnitude.  Returns a range error if it overflows.
    ///
    /// ```
    /// use ledpwm5947::pwm::{Step, RangeError};
    ///
    /// let small_step = Step::new(10);
    /// let doubled = small_step.double().expect("It should double");
    ///
    /// let big_step = Step::new(3000);
    /// if let Err(v) = big_step.double() {
    ///     assert_eq!(RangeError::Overflow, v);
    /// } else {
    ///     assert!(false, "Should have returned an error");
    /// }
    /// ```
    pub fn double(&self) -> Result<Self, RangeError> {
        let doubled_value = self.amount << 1;
        Self::checked_new(doubled_value)
    }

    /// Cuts the size of the step in half.  It is not expected to overflow
    /// or underflow.
    ///
    /// ```
    /// use ledpwm5947::pwm::Step;
    ///
    /// let step = Step::new(20);
    /// let half_step = Step::new(10);
    ///
    /// assert_eq!(half_step, step.half_step())
    /// ```
    pub fn half_step(&self) -> Self {
        Step {
            amount: self.amount / 2,
        }
    }

    /// Cuts the size of a step to one-forth its size.  It uses integer arithmetic
    /// and so remainders are truncated.
    ///
    /// ```
    /// use ledpwm5947::pwm::Step;
    /// let step = Step::new(20);
    ///
    /// assert_eq!(Step::new(5), step.quarter_step());
    ///
    /// let step = Step::new(22);
    /// assert_eq!(Step::new(5), step.quarter_step());
    /// ```
    pub fn quarter_step(&self) -> Self {
        Step {
            amount: self.amount / 4,
        }
    }

    /// Cuts the size of a step into one-eighth its size.  It uses integer
    /// arithmetic, so the remainders are truncated.
    ///
    /// ```
    /// use ledpwm5947::pwm::Step;
    ///
    /// let step = Step::new(32);
    /// assert_eq!(Step::new(4), step.eighth_step());
    ///
    /// let step = Step::new(255);
    /// assert_eq!(Step::new(31), step.eighth_step());
    /// ```
    pub fn eighth_step(&self) -> Self {
        Step {
            amount: self.amount / 8,
        }
    }

    /// Cuts the size of a step into one-sixteenth its size.  It uses integer
    /// arithmetic, so the remainders are truncated.
    ///
    /// ```
    /// use ledpwm5947::pwm::Step;
    ///
    /// let step = Step::new(32);
    /// assert_eq!(Step::new(2), step.sixteenth_step());
    ///
    /// let step = Step::new(255);
    /// assert_eq!(Step::new(15), step.sixteenth_step());
    /// ```
    pub fn sixteenth_step(&self) -> Self {
        Step {
            amount: self.amount / 16,
        }
    }
}

impl core::ops::Add for Step {
    type Output = Result<Self, RangeError>;

    /// Add two steps together.  The sum can overflow or underflow if the total
    /// step size exceeds the PWM maximum value.
    ///
    /// ```
    /// use ledpwm5947::pwm::{Step, RangeError};
    ///
    /// let step1 = Step::new(15);
    /// let step2 = Step::new(14);
    ///
    /// let result = (step1 + step2).expect("It should add the two values");
    /// assert_eq!(Step::new(29), result);
    ///
    /// let step1 = Step::new(4000);
    /// let step2 = Step::new(2000);
    ///
    /// if let Err(v) = step1 + step2 {
    ///     assert_eq!(RangeError::Overflow, v);
    /// } else {
    ///     assert!(false, "Should have returned an error");
    /// }
    /// ```
    fn add(self, rhs: Step) -> Self::Output {
        let computed_value = self.amount + rhs.amount;
        if computed_value < 0 {
            if -computed_value > PWM_MASK as i16 {
                Err(RangeError::Underflow)
            } else {
                Ok(Step {
                    amount: computed_value,
                })
            }
        } else {
            if computed_value > PWM_MASK as i16 {
                Err(RangeError::Overflow)
            } else {
                Ok(Step {
                    amount: computed_value,
                })
            }
        }
    }
}

impl core::ops::Sub for Step {
    type Output = Result<Self, RangeError>;

    /// Implements subtraction for a step
    ///
    /// ```
    /// use ledpwm5947::pwm::{Step, RangeError};
    ///
    /// let step1 = Step::new(50);
    /// let step2 = Step::new(25);
    ///
    /// assert_eq!(Step::new(-25), (step2 - step1).expect("It should subtract two steps"));
    ///
    /// let step1 = Step::new(-2500);
    /// let step2 = Step::new(2500);
    ///
    /// let step3 = step1 - step2;
    ///
    /// if let Err(v) = step1 - step2 {
    ///     assert_eq!(v, RangeError::Underflow);
    /// } else {
    ///     assert!(false, "It should have raised an error");
    /// }
    /// ```
    fn sub(self, rhs: Step) -> Self::Output {
        let computed_value = self.amount - rhs.amount;
        if computed_value < -4095 {
            Err(RangeError::Underflow)
        } else if computed_value > 4095 {
            Err(RangeError::Overflow)
        } else {
            Ok(Step {
                amount: computed_value,
            })
        }
    }
}

impl PWMValue {
    /// Returns a new PWM value given a number.  If the value is greater than
    /// PWM max, it is set to max, if it is less than min, it is set to min.
    ///
    /// ```
    /// use ledpwm5947::pwm::PWMValue;
    ///
    /// let p1 = PWMValue::new(27);
    /// assert_eq!(PWMValue::new(27), p1);
    ///
    /// let p1 = PWMValue::new(5000);
    /// assert_eq!(PWMValue::max(), p1);
    ///
    /// let p1 = PWMValue::new(-5000);
    /// assert_eq!(PWMValue::min(), p1);
    /// ```
    pub fn new(v: i32) -> Self {
        if v > PWM_MASK as i32 {
            PWMValue::max()
        } else if v < 0 {
            PWMValue::min()
        } else {
            PWMValue { raw: v as i16 }
        }
    }

    /// Returns the minimum PWM setting, in this case it's zero.
    ///
    /// ```
    /// use ledpwm5947::pwm::PWMValue;
    ///
    /// let min = PWMValue::min();
    /// let p1 = PWMValue::new(1);
    /// let p2 = PWMValue::new(0);
    /// let p3 = PWMValue::new(-1);
    ///
    /// assert!(p1 != min, "1 should not be min");
    /// assert!(p2 == min, "Zero should be also min");
    /// assert!(p3 == min, "-1 should also clamp to min");
    /// ```
    pub fn min() -> Self {
        PWMValue { raw: 0 }
    }

    /// Returns the maximum PWM setting, in this case it's 4095.
    ///
    /// ```
    /// use ledpwm5947::pwm::PWMValue;
    ///
    /// let max = PWMValue::max();
    /// let p1 = PWMValue::new(4094);
    /// let p2 = PWMValue::new(4095);
    /// let p3 = PWMValue::new(4096);
    ///
    /// assert!(p1 != max, "4094 should be less than max");
    /// assert!(p2 == max, "4095 should be the max for 12 bit");
    /// assert!(p3 == max, "4096 should clamp to max");
    /// ```
    pub fn max() -> Self {
        PWMValue { raw: 0x0FFF }
    }

    pub(crate) fn bits(&self) -> [bool; 12] {
        let mut result: [bool; 12] = [false; 12];

        for i in 0..12 {
            result[i] = PWM_BIT_MASKS[i] & (self.raw as u16) > 0;
        }

        result
    }
}

impl core::default::Default for PWMValue {
    /// Default PWM value should be off, or zero.
    ///
    /// ```
    /// use ledpwm5947::pwm::PWMValue;
    ///
    /// let v = PWMValue::default();
    ///
    /// assert_eq!(PWMValue::new(0), v);
    /// ```
    fn default() -> Self {
        PWMValue::min()
    }
}

impl core::default::Default for Step {
    /// The default value for a step should be 1.
    ///
    /// ```
    /// use ledpwm5947::pwm::Step;
    ///
    /// let s = Step::default();
    /// assert_eq!(Step::new(1), s);
    /// ```
    fn default() -> Self {
        Step { amount: 1 }
    }
}

impl core::ops::Add<Step> for PWMValue {
    type Output = Result<Self, RangeError>;

    /// PWM values are altered by stepping them up or down.  The current value
    /// is stepped up or down by a  positive or negative step.  The resulting
    /// value can underflow or overflow.
    ///
    /// ```
    /// use ledpwm5947::pwm::{PWMValue, Step, RangeError};
    ///
    /// let p1 = PWMValue::new(100);
    /// let s1 = Step::new(10);
    ///
    /// let p2 = p1 + s1;
    /// assert_eq!(PWMValue::new(110), p2.expect("It should equal 110"));
    ///
    /// let p1 = PWMValue::new(100);
    /// let s1 = Step::new(-10);
    ///
    /// let p2 = p1 + s1;
    /// assert_eq!(PWMValue::new(90), p2.expect("It should equal 90"));
    ///
    /// let p1 = PWMValue::new(4500);
    /// let s1 = Step::new(500);
    ///
    /// if let Err(v) = p1 + s1 {
    ///     assert_eq!(RangeError::Overflow, v);
    /// } else {
    ///     assert!(false, "It should have raised an error");
    /// }
    ///
    /// let p1 = PWMValue::new(-2500);
    /// let s1 = Step::new(-2500);
    ///
    /// if let Err(v) = p1 + s1 {
    ///     assert_eq!(RangeError::Underflow, v);
    /// } else {
    ///     assert!(false, "It should have raised an error");
    /// }
    /// ```
    fn add(self, rhs: Step) -> Self::Output {
        let computed_value = self.raw + rhs.amount;
        if computed_value < 0 {
            Err(RangeError::Underflow)
        } else if computed_value > PWM_MASK as i16 {
            Err(RangeError::Overflow)
        } else {
            Ok(PWMValue {
                raw: computed_value,
            })
        }
    }
}

impl Iterator for PWMValue {
    type Item = PWMValue;

    fn next(&mut self) -> Option<PWMValue> {
        if self.raw < 4095_i16 {
            self.raw += 1;
            Some(PWMValue { raw: self.raw })
        } else {
            None
        }
    }
}

impl From<u8> for PWMValue {
    fn from(val: u8) -> Self {
        let shifted = (val as i16) << 4;
        match val {
            0 => PWMValue { raw: shifted },
            1..=15 => PWMValue {
                raw: shifted | 0x0001,
            },
            16..=31 => PWMValue {
                raw: shifted | 0x0002,
            },
            32..=47 => PWMValue {
                raw: shifted | 0x0003,
            },
            48..=63 => PWMValue {
                raw: shifted | 0x0004,
            },
            64..=79 => PWMValue {
                raw: shifted | 0x0005,
            },
            80..=95 => PWMValue {
                raw: shifted | 0x0006,
            },
            96..=111 => PWMValue {
                raw: shifted | 0x0007,
            },
            112..=127 => PWMValue {
                raw: shifted | 0x0008,
            },
            128..=143 => PWMValue {
                raw: shifted | 0x0009,
            },
            144..=159 => PWMValue {
                raw: shifted | 0x000A,
            },
            160..=175 => PWMValue {
                raw: shifted | 0x000B,
            },
            176..=191 => PWMValue {
                raw: shifted | 0x000C,
            },
            192..=207 => PWMValue {
                raw: shifted | 0x000D,
            },
            208..=223 => PWMValue {
                raw: shifted | 0x000E,
            },
            _ => PWMValue {
                raw: shifted | 0x000F,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_pwm() {
        let v1 = PWMValue::new(30);
        assert_eq!(30, v1.raw);

        let v2 = PWMValue::new(-1);
        assert_eq!(0, v2.raw);

        let v3 = PWMValue::new(5000);
        assert_eq!(4095, v3.raw);
    }

    #[test]
    fn test_min_max_defualt() {
        let min = PWMValue::min();
        assert_eq!(min.raw, 0);

        let max = PWMValue::max();
        assert_eq!(max.raw, 0xfff);

        let default = PWMValue::default();
        assert_eq!(default.raw, 0);
    }

    #[test]
    fn test_create_step() {
        let negative_step = Step::new(-10);
        assert_eq!(-10, negative_step.amount);

        let positive_step = Step::new(10);
        assert_eq!(10, positive_step.amount);

        let big_negative = Step::new(-5000);
        assert_eq!(-4095, big_negative.amount);

        let big_positive = Step::new(5000);
        assert_eq!(4095, big_positive.amount);
    }

    #[test]
    fn test_reverse_step() {
        let positiive_step = Step::new(10);
        let reversed = positiive_step.reverse();
        assert_eq!(-10, reversed.amount);

        let reversed = reversed.reverse();
        assert_eq!(10, reversed.amount);
    }

    #[test]
    fn test_step_subtraction() {
        let step1 = Step::new(-2500);
        let step2 = Step::new(2500);

        let step3 = step1 - step2;
        match step3 {
            Err(v) => assert_eq!(RangeError::Underflow, v),
            Ok(_) => assert!(false, "should have returned an error"),
        }
    }

    #[test]
    fn test_simple_iteration() {
        let mut last_value = PWMValue::default();
        let mut counter = 0;

        for i in PWMValue::min().take(4) {
            last_value = i;
            counter += 1;
        }
        assert_eq!(4, counter);
        assert_eq!(PWMValue::new(4), last_value);
    }

    #[test]
    fn test_end_of_iteration() {
        let mut current = PWMValue::new(4094);
        current = current.next().unwrap_or_default();
        assert_eq!(PWMValue::new(4095), current);

        current = current.next().unwrap_or_default();
        assert_eq!(PWMValue::default(), current);
    }

    #[test]
    fn test_other_methods() {
        let current = PWMValue::default();
        assert_eq!(PWMValue::new(4095), current.last().unwrap_or_default());

        let current = PWMValue::default();
        let mut stepping_iterator = current.step_by(4);
        assert_eq!(
            PWMValue::new(17),
            stepping_iterator.nth(4).unwrap_or_default()
        );
    }

    #[test]
    fn test_from_u8() {
        let test_cases = &mut [
            (0_u8, PWMValue::min()),
            (1_u8, PWMValue::new(0x11)),
            (16_u8, PWMValue::new((16 << 4) + 2)),
            (32_u8, PWMValue::new((32 << 4) + 3)),
            (48_u8, PWMValue::new((48 << 4) + 4)),
            (128_u8, PWMValue::new((128 << 4) + 9)),
            (255_u8, PWMValue::max()),
        ];

        for case in test_cases {
            assert_eq!(case.1, PWMValue::from(case.0));
        }
    }
}
