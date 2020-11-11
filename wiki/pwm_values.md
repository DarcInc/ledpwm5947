# PWM Values
One thing that doesnt' feel right is the use of unsigned integers for the PWM values.
For one thing, we could pass something like 4128 and the library will set the value.
This feels like it should be an out of range condition.
If you add intensity to a color, and the maximum allowable value is 4095, then there should be an error.
The current implementation just clamps it to a 12-bit value.

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

## What Should A Lighting PWM Value Do?
A long time ago in a systems programming class far, far away, we talked about abstract types.
An abstract data type, roguhly, is platonic form of a data type.
If we were implementing a string data type, for example, there are some generally stringy things to do.
We might debate exactly what that should be, and the context in which we operate may limit our options.

* Get its length in storage
* Get its length in printable characters
* Search for a sub-string in the string
* Replace a sub-string in the string
* Make a new string from concatenated strings
* and so on...

On an embedded device, for example, we might not care much about string handling.
We may also limit ourselves to 7-bit ASCII, if that's what our peripherals support.
In a web application, these issue may be more significant.
Regardless, we first figure out what we need to do and then figure out how to represent the type.

We should also hide the actual implementation from the library user.
Right now I'm using a 12-bit PWM, but what if I want to port the library to a 10-bit PWM?
What if I discover a more efficient way to handle the values I send to the board?
Ideally, the choice of data type should be transparent to the library user.
In fact, I want them to focus more on lighting that 12 bit math.

### Min and Max and Default
One feature I would like is to get the minimum and maximum PWM values.
This way I'm not tied to using a zero.
Funny story, I wrote zero as `0x0_16` in my code instead of `0x0_u16`.
The former is the decimal value 22, while the latter is the decimal value 0.
For hours I couldn't figure from where the value 22 came, as I stepped through the code.
Also, I would be less likely to make a mistake specifying the maximum value.

### Adding And Differencing
As someone who's made fairy wings in the past, fades from one color to another require math.
If I have two colors, and want to fade from one to the other, I will need to incrementally add changes.
I can certaily do this with unsigned 16 bit integers, although there's an overflow and underflow risk.
Maybe I take input from a button and the user mashes the button and we mistakenly overflow.
I would also like to be able to retain the difference as a negative number.

### Ranges
It also be nice to write ranges over PWM values.
This might even replace some of the math burden.
Being able to fade by writing simple iteration would eliminate a lot of boilerplate math.

### Converting From u8
There is a lot of information about color using 8-bit hexidecimal numbers for red green and blue.
For example, on the web, 0xFFFF00 is a bright yellow.
In order to better support the notion of colors, it would be nice to convert from `u8`.
This will make it easier to express RGB colors.

## Implementation
The next step is to implement these features.
My first instinct was to use a "unit" type, thinking it would be more compact.
That's not true.
Rust gaurantees the size of a struct is simply the size of its components.
Note that the fields in the struct aren't public.
This discourages binding to the internal representaion.

```rust
// These two should be the same size
pub struct PWMUnit(i16);

pub struct PWMStruct {
    raw: i16,
}
```

Note that I've switched to a signed integer from an unsigned integer.
I'm no longer worried about being passed a negative number.
This will probably make math easier, but I'm not sure yet.

My first cut at this type includes the concept of underflow and overflow.
For now, I'd like to explore the idea that creating a PWM value with an out of range value is an error.

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

### Implementing Min and Max and Default
implementing the concepts 

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
