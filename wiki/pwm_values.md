# PWM Values
Using unsigned or signed integers for PWM values seems like a gateway for errors.
For example, let's say you're bringing the LED up from 0 to 100%.
You start at 0 and add a fixed amount like `0x0F` with each step.
You're careful and write only the lowest 12 bits of an unsigned integer to the device.
Because you forgot to check to see if you had exceeded 4095, the light starts jumping around.
When you get to 4095 (111111111111) and add 16, the lowest 12 bits are 000000001111.
You will see the light brighten and suddenly dim, until you hit an overflow.

```rust
/// Writes a value into the given channel.  It basically updates the buffer
/// of values, making sure the passed in value a 12-bit integer by making it
/// with the PWM_MASK, above.  We also don't worry about values outsie the
/// range of 24, but silently.
pub fn write_pwm(&mut self, channel: &usize, pwm_value: &u16) {
    if *channel < 24 {
        self.buffer[*channel] = *pwm_value & PWM_MASK;
    }
}
```

A lot of the Rust libraries I've looked into have a common property.
They try to take away the 'foot-gun.'
A foot-gun is a capability that allows you to shoot yourself in the foot.
For example, in C I have complete control over the allocation and freeing of memory.
But it also allows for a wide range of nasty bugs from simple memory leaks to leaking protected data.
It seems like using raw integers is powerful (we can do arbitrary math and big-bang those values), but it's foot-gun.

## What Should A Lighting PWM Value Do?
A long time ago in a systems programming class far, far away, I learned about abstract types.
An abstract type is really about the operations allowed on a given type, without referencing a particular implementation.
Using strings as an example, we might store the data in a variety of ways.
Strings can be stored like C strings with a null-terminated array.
They can be stored as in Pascal, by tracking an array of memory and its length.
The values could be in ASCII, UTF-8, or your own multi-byte encoding.
Regardless, there are "stringy" behaviors:

* Some notion of string length.
* Search for a sub-string in the string.
* Replace a sub-string in the string.
* Make a new string from concatenated strings.
* And so on...

Let's say that I find a similar device that has a 10-bit PWM but 128 instead of just 24 channels.
If I use a reasonably abstract notion of a PWM value, I might be able to re-use more code.
The applications using my code wouldn't be dependent on the exact representation.
A value could be 8, 10, 12, or 16 bits.
Hopefully, a lot of the existing code could just switch to the new library.

### Min and Max and Default
Thinking about the code I would write, I might want to start from the minimum value to the maximum value.
In this case I know those are 0 and 4095, respectively, but that's an implementation detail.
It also makes sense to have a 'default' value such as the minimum value.
As an example of the problems of using integers, I wrote zero as `0x0_16` in my code instead of `0x0_u16`.
The Rust compiler interprets the former is the decimal value 22, while the latter is the decimal value 0.
For hours, I couldn't figure from where the value 22 came, as I stepped through the code.
It would be safer to simply use ```max()```, ```min()```, and ```default()``` to get those values.

### Adding And Differencing
As someone who's made fairy wings in the past, I can tell you color fades are important.
Fading from one color to another requires math.
I can certainly perform the math on 16 bit integers, but there's an overflow and underflow risk.
It would be safer to increment or decrement abstract values and avoid the overflow exception.
I would also like to be able to retain the difference as a negative number.
It is a foot-gun to mix signed and unsigned arithmetic.
Better to do it once, correctly, on the abstract type.

### Ranges
It also be nice to write ranges over PWM values.
This might even replace some of the math burden.
Being able to fade by writing simple iteration would eliminate a lot of boilerplate math.

### Converting From u8
There is a lot of information about color using 8-bit hexidecimal numbers for red green and blue.
For example, on the web, 0xFFFF00 is a bright yellow.
In order to better support the notion of colors, it would be nice to convert from `u8`.
This will make it easier to express RGB colors.
I can also repurpose all that code I've written to perform HSV to RGB conversions, etc.
That really belongs in another library, and this one just needs to a good conversion from `u8`.

## Implementation
The next step is to implement these features.
My first instinct was to use a "unit" type, thinking it would be more compact.
That's not true.
Rust guarantees the size of a struct is simply the size of its components.
Note that the fields in the struct aren't public.
This discourages binding to the internal representation.
The Unit type exposes the underlying value.

```rust
// These two should be the same size
pub struct PWMUnit(i16);

pub struct PWMStruct {
    raw: i16,
}
```

Note that I've switched to a signed integer from an unsigned integer.
I'm no longer worried about being passed a negative number.
Using a signed integer will probably make math easier, but I'm not sure yet.
I just need to remember to 'clamp' my values to the range 0 to 4095, inclusive.
My first cut at this type includes the concept of underflow and overflow.
I don't want a panic.
I will need to range check the values and then issue the appropriate error.


```rust
#[derive(Copy, Clone, PartialOrd, PartialEq)]
pub struct PWMValue {
    raw: i16,
}

pub enum RangeError {
    Underflow,
    Overflow,
}

impl PWMValue {
    pub fn new(v: i32) -> Result<Self, RangeError> {
        if v < 0 {
            Err(RangeError::Underflow)
        } else if v > 0x0fff {
            Err(RangeError::Overflow)
        } else {
            Ok(
                PWMValue {
                    raw: v as i16,
                }
            )
        }
    }
}
```

For the ```new()``` function I think it would be best to take a wide berth of values.
Even if I restrict the parameter to unsigned, 16-bit values, I might still have an overflow.
I wasn't sure if returning a `Result` enumeration was a good idea from the `new()` function.
There are examples of other Rust libraries providing constructors that can return an error.
I also felt that passing a value outside of allowable range is an error.
Maybe the calling function would happily attempt to create ```PWMValue```s in a loop, not realizing it had surpassed the limit for PWM values.
If I just clamped the values to a legal range, I could be abetting the hidiing of an error.

### Implementing Min and Max and Default
These are fairly straightforward.
I elided the comments for brevity, but min and max simply create the appropriate 12-bit values.
A 10-bit PWM would use `0x03FF` as its maximum value.
The ```defualt()``` value can simplify error handling.
For example, if we use the `or_default()` family of functions.

```rust
impl PWMValue {
    pub fn min() -> Self {
        PWMValue { raw: 0 }
    }

    pub fn max() -> Self {
        PWMValue { raw: 0x0FFF }
    }
}


impl core::default::Default for PWMValue {
    fn default() -> Self {
        PWMValue::min()
    }
}
```

### Iteration Instead of Ranges
When I went to implement ranges, I found they are part of the nightly Rust build.
I would rather pin my code to the stable build.
However, I was able to introduce iteration.
Iteration introduces key methods, including ```take()``` and ```filter```.
These are the "functional" methods on a series.

```rust
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
```

### Conversion from u8
For the `u8` conversion, I wanted a smoother cline through the values than just multiplying by 16.
Also, 255 in `u8` should be the maximum value for a PWM value.
Instead of just shifting left 4 bits (multiplying by 16), I also added a fractional amount to smooth the transition.
Zero should always be PWM 'off'.

```rust
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
            // Additional cases elided for brevity ...
            208..=223 => PWMValue {
                raw: shifted | 0x000E,
            },
            _ => PWMValue {
                raw: shifted | 0x000F,
            },
        }
    }
}
```

### Steps Became a Thing
One problem I came across was adding a fixed amount to a PWMValue.
In my mind it didn't make sense to add a PWM value to a PWM value.
A PWM value is a fixed frequency for the PWM.
It made more sense to have an amount that's a change in frequency.
That resulted in `Step` being added as a concept.

Let's say I want to bring up a light from zero to full brightness.
The initial `PWMValue` is zero.
Each 50ms, I bump it 128, so after about 1.6 seconds, I'm at full brightness.
I don't want to expose the underlying implementation.
I also didn't want to just add raw numbers.
Remember, we might want to use a different resolution PWM in the future.

## Doc Tests vs Mod Test
One interesting note is that I find doc tests just as effective in the initial development.
It both documents the intent of the function and verifies its compliance with that intent.
Going forward, I think I will prefer doc tests over module tests.
Module tests may be reserved more for validating bug fixes and making sure there are no regressions in the future.

During the initial development I'm interested in basic cases.
Happy path - does the API do what it's supposed to when all the inputs are reasonable?
Simple bad path - given expected "bad" inputs, does it raise the expected errors?
As I started documenting the code I quickly realized my doc tests would duplicate the tests in the test module.
This crates two distinct things to maintain, but offer no new capability.

In general, happy path and expected bad path are not enough.
I'm limited in both imagination and time when creating the initial implementation.
There may be edge cases or situations I have not considered.
Have I covered every case in which an expected invariant may be violated?
What if someone misuses some API in a novel way?
The type system attempts to prevent misuse, but only if I design the types correctly.

In theory, unforseen edge cases and misuse of the API will result in a bug.
That bug, when resolved, should become a module test.
Fuzzing the input attempts to drive out these edge cases when those cases are not obvious.
However, these edge cases are bugs and don't fit into documentation comments.
Therefor they should go into module comments.

## Taking Stock
At this point I'm much happier with the error handling.
Using the PWM values feels much safer than raw numbers.
The features provided seem to be rich and safe to use.
I can easily iterate from one value of illumination to another value.

There are some things that are still a little off.
First is the notion of just creating PWM pins.
This specific board has 24 pins only.
I should be able to constrain the user from creating PWM pin 25.
I started that in this round of changes.
I meant to put that off until later, but the problem bothered me too much.
This will be the next focus.

Next is the code organization for steps.
It feels like maybe Steps belong in their own module.
However, they are intimately tied to the PWM value.
It might make more sense to keep them in the existing module.
I'm not convinced there's a problem, but I'm suspect.