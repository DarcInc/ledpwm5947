# Error Handling
Let's start with version [0.0.1](https://github.com/DarcInc/ledpwm5947/tree/0.0.1).
This code came from a loose, incomplete translation of the C API from [Adafruit](https://github.com/adafruit/Adafruit_TLC5947).
I translated just enough code to turn on and off some lights.
It does not do everything the existing C library for Arduino does.
It does not implement multiple, chained 5947 devices, for example.
As a translation from C, it is possibly not good Rust code.

Looking at my library versus other Rust libraries, the error handling stood out.
Not only the warnings I received from the compiler, but the code style as ell.
The first task is to get rid of the compiler warnings when ```Result``` is returned but not handled.
Giving users of the library a chance to respond to errors seems reasonable.
They may choose to turn on a big red warning light if some task fails.

To first get rid of the compiler warnings, I used a ```_res``` place holder variable.
The warning goes away because I use the function result and the leading underscore indicates I don't intend to use the value.
In debugging the problems with the initial version, I stepped through with the debugger to see if ```_res``` ever returned an error.
Relying on the user to run in a debugger not ideal.
This style certainly does not appear in the Rust code I've read so far.

## Setting Pins
The specific board I'm testing with uses the [stm32l4xx_hal](https://github.com/stm32-rs/stm32l4xx-hal) crate.
The abridged code below implements the ```OutputPin``` interface in the stm32l4xx_hal crate.
It can return the ```Infallible``` error.
```Infallible``` is an error placeholder that basically says 'I never expect this to fail.'
That sounds fairly reasonable in this circumstance.
Setting the pin to high or low basically writes a value to a register and that will never fail.
For this board, the ```set_high``` and ```set_low``` operations will never fail.

```rust
impl<MODE> OutputPin for $PXx<Output<MODE>> {
    type Error = Infallible;


    fn set_high(&mut self) -> Result<(), Self::Error> {
        // NOTE(unsafe) atomic write to a stateless register
        unsafe { (*$GPIOX::ptr()).bsrr.write(|w| w.bits(1 << self.i)) }
        Ok(())
    }


    fn set_low(&mut self) -> Result<(), Self::Error> {
        // NOTE(unsafe) atomic write to a stateless register
        unsafe { (*$GPIOX::ptr()).bsrr.write(|w| w.bits(1 << (16 + self.i))) }
        Ok(())
    }
}
```

Why does the stm32l4xx_hal implementation return an error at all?
Because the trait derives from the ```OutputPin``` trait in the 0.2.x version of [embedded_hal](https://github.com/rust-embedded/embedded-hal/blob/384b4934a88a7939604ca7cf4048c49a4cd07c16/src/digital/v2.rs#L6-L21).
(I removed the comments in the code from that crate in the interests of space.)
Note that the link is to a specific branch.
Version 1.0.0 is already in alpha and has a somewhat different definition for this trait.
I'll continue to track with the version used by the sm32l4xx_hal crate.

```rust
pub trait OutputPin {
    type Error;
    fn set_low(&mut self) -> Result<(), Self::Error>;
    fn set_high(&mut self) -> Result<(), Self::Error>;
}
```

When implementing this interface, the author can specify a concrete error type.
Other implementations for other boards may return an error other than ```Infallible```.
However, I won't know the specific type of error.
That depends on the board I'm using.
Writing a library I intend to be portable beyond the stm32l432, I will need to think about the error handling.

One option might be to match on any possible error.
The ```Err(_)``` arm matches on any error and discards the specific error information.
That would allow the library to work if the board returns some other error besides ```Infallible```.
I could return ```str``` error as the error type.
A user with an attached debugger or writing the error to a serial line would be better able to diagnose the problem.
This solution does not feel right.
For example, match blocks feel a little verbose.

```rust
pub fn begin(&mut self) -> Result<(), &'static str>
{
    match self.oe.set_low() {
        Err(_) => return Err("Failed to set OE line"),
        _ => (),
    };

    match self.latch.set_low() {
        Err(_) => return Err("Failed to set Latch line"),
        _ => (),
    }

    match self.data.set_low() {
        Err(_) => return Err("Faled to set Data line"),
        _ => (),
    }

    match  self.clock.set_low() {
        Err(_) => return Err("Failed to set Clock line"),
        _ => (),
    }

    for i in 0..24 {
        self.buffer[i] = 0x0_u16;
    }

    Ok(())
}
```

Chaining the ```set_low()``` calls together with ```and_then``` calls does not work.
Each pin is a unique type which means it has a unique implementation of ```OutputPin```.
Therefore, the compiler can't guarantee each error isn't a different type.
Remember that the error type is a generic parameter in the ```OutputPin``` trait for the embedded_hal crate.
Even if two output pins are exactly the type of thing (in human parlance), they are still distinct types to the compiler.

## Is The Abstraction Right
One code smell when using ```match``` code above is it relies on strings.
I suspect I'm missing a concrete error type or abstraction.
More than that, I'm missing the level of indirection that unifies the output pin error types.
I would like to use a common error to make error propagation easier.

For example, if a board raises an error on a pin, it might make sense to reset the pin.
Naming the pin in a string isn't very ergonomic to the library user.
By having a common error type, I'll also be able to propagate errors using the ```?``` operator.
That would allow me to write the ```begin``` function, more simply.

```rust
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
```

### A Pin Error
From an ergonomic standpoint, I think I understand what error structure I would like.
It should have two properties that allow me to pin-point errors.
The first should identify which pin failed.
The second should provide additional debugging details.
The first property by itself should easily allow code using this library to identify the pin that failed.
The second is for developer ergonomics.

To run this particular board I need four pins.
The first is a latch pin th indicate I'm writing data to the board.
Next is the data pin, which I toggle to write bits on the controller.
The OE pin allows the PWM pins to be quickly disabled or enabled.
Finally, there's a clock pin to allow me to control the data flow.
A pin can have one of these four roles.

```rust
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
```

The ```PinError``` type matches the role a pin fulfills to an error message for that pin.
A constructor allows me to create a ```PinError``` if I know the pin's purpose and an error string.
I expect most "fixed" error strings in code to have static lifetimes.

This is my first shot at the better error reporting.
I've seen other Rust libraries wrap a given error in another error.
However, I don't know the error type a specific board will return as an error.
It could have its own error types.
That's why I don't use the wrapping pattern here.
Instead I define my own semantic error.

### A Type Wrapper
I next turn my attention to the actual pins themselves.
I need a generic ```PWMPin``` to standardize the expected error type from a given pin.
Since each output pin in the [enbedded_hal](https://github.com/rust-embedded/embedded-hal/blob/d45af5f9bf25e7e29029db578aaa3c10c35da424/src/digital.rs#L44) is a generic type, the four pins I need could return arbitrary error types. 
The ```PWMPin``` wrapper will deal with the different returned error types.
Since I also need it to track the pin's role for error reporting purposes, I'll need that as well.

```rust
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
```

I did not do is expose this to my library's users.
I don't think they will need to use it, directly.
I would rather let them work with their own pin types for their boards.
I don't want to force them to adopt an 'alien' type just to use the library.
It should be as transparent as using any other device on their board.

Implementing the `OutputPin` trait is fairly straightfoward.
I match on the raw pin's result.
I return ```Ok``` or ```Err``` as needed, indicating which pin failed.

```rust
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
```

From a library user's perspective, the interface is fairly simple.
In the construct for the ```PWM5947``` interface, they pass in their definition of ```OutputPin```.
Those are wrapped in the ```PWMPin```.
The the type wrapper now returns a consistent error type.

```rust
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
}
```

The original implementation of the ```PWM5947``` swallowed the errors to just get rid of compiler wwarnings.
The new ```PWM5947``` implementation returns an error that includes which pin failed.
For example, the code below flushes the bits in the buffered PWM values to the device.
Since each pin returns a common error type (```PinError```) from its wrapper type, I can use the `?` operator.
The remaining methods are similar.
The error handling and implementation look much tighter.

```rust
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
```

If some user of this library called ````flush()```, and there was an unexpected error toggling the data pin, they could recover.
The only down-side is that I lose any board specific error information.
Of course, this is only the first round of cleanup and nothing is set in stone.
We might be able to preserve that information.
For now, the error handling is no longer the biggest eyesore.

## Next Steps
The next item I find irksome is using raw integer values to set the PWM.
For one thing, the PWM is 12-bit, meaning only values from 0 to 4096 are valid.
A 16-bit signed or unsigned integer can legitimately represent "out of range" values for the PWM.


