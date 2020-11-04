//! The PWM module encapsulates the PWM values passed to the 5947.  This shouldn't 
//! be confused with the PWM from the hardware hal or processor specific hal.
//! 


/// The PWM_MASK allows us to "mask" the extra bits in a 16 bit integer, which
/// is what we're using to store the PWM state.  Since this is used internally,
/// to clamp values to a valid 12-bit number, we don't need to export it.
pub const PWM_MASK: u16 = 0x0fff;

/// The PWM value is a number between 0 and the maximum 12-bit value.  As an 
/// invariant, the PWM value can never be below 0 or above 4095.
#[derive(Copy, Clone, PartialOrd, PartialEq)]
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
                amount: PWM_MASK as i16
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
            Ok(Step {
                amount
            })
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
        let temp = self.amount << 1;
        Self::checked_new(temp)
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

    fn add(self, rhs: Step) -> Self::Output {
        let temp = self.amount + rhs.amount;
        if temp < 0 {
            if -temp > PWM_MASK as i16 {
                Err(RangeError::Underflow)
            } else {
                Ok(Step {
                    amount: temp,
                })
            }
        } else {
            if temp > PWM_MASK as i16 {
                Err(RangeError::Overflow)
            } else {
                Ok(Step {
                    amount: temp,
                })
            }
        }
    }
}

impl core::ops::Sub for Step {
    type Output = Result<Self, RangeError>;

    fn sub(self, rhs: Step) -> Self::Output {
        let temp = self.amount - rhs.amount;
        if temp < -4095 {
            Err(RangeError::Underflow)
        } else if temp > 4095 {
            Err(RangeError::Overflow)
        } else {
            Ok(Step {
                amount: temp,
            })
        }
    }
}

impl PWMValue {
    pub fn new(v: i32) -> Self {
        if v > PWM_MASK as i32 {
            PWMValue::max()
        } else if v < 0 {
            PWMValue::min()
        } else {
            PWMValue {
                raw: v as i16,
            }
        }
    }

    pub fn min() -> Self {
        PWMValue {
            raw: 0,
        }
    }

    pub fn max() -> Self {
        PWMValue {
            raw: 0x0FFF,
        }
    }
}

impl core::default::Default for PWMValue {
    fn default() -> Self {
        PWMValue::min()
    }
}

impl core::default::Default for Step {
    fn default() -> Self {
        Step {
            amount: 1,
        }
    }
}

impl core::ops::Add<Step> for PWMValue {
    type Output = Result<Self, RangeError>;

    fn add(self, rhs: Step) -> Self::Output {
        let temp = self.raw + rhs.amount;
        if temp < 0 {
            Err(RangeError::Underflow)
        } else if temp > PWM_MASK as i16  {
            Err(RangeError::Overflow)
        } else {
            Ok(PWMValue {
                raw: temp,
            })
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
}