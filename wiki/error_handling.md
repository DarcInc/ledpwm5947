# Error Handling
Let's start with version [0.0.1](https://github.com/DarcInc/ledpwm5947/tree/0.0.1).
It came from a loose translation of the C API, albeit an incomplete translation.
It was just enough to make sure the basic protocol works.
It did not implement multiple, chained 5947 devices, for example.
As a translation from C, it is possibly not good Rust code.

The most glaring problem to me was error handling.
I struggled with that.
I wanted to get rid of the compiler warnings when ```Result``` was returned but not handled.
In a more general sense, when there's a possible side-effect, we should handle errors.
For example, if the controller tries to read the i2c bus but fails, maybe we need to turn on a big red light.
In the interim, I used a ```_res``` place holder to catch the error so I could review it in the debugger, but basically discard it.
This is not ideal in any language and does not appear in the Rust code I've read so far.

## Setting Pins
The specific board I'm testing with uses the stm32l4xx_hal crate.
It implements the ```OutputPin``` interface, returning the ```Infallible``` error.
For my board, the ```set_high``` and ```set_low``` operations are never going to report anything than ```Ok```.

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

Why does this return an error at all?
It's because the trait is based on the ```OutputPin``` trait defined in the 0.2.x version of [embedded_hal](https://github.com/rust-embedded/embedded-hal/blob/384b4934a88a7939604ca7cf4048c49a4cd07c16/src/digital/v2.rs#L6-L21).
(I removed the comments in the interests of space.)
Note that the link is to a specific branch.
Version 1.0.0 is already in alpha and has a somewhat different definition for this trait.

```rust
pub trait OutputPin {
    type Error;
    fn set_low(&mut self) -> Result<(), Self::Error>;
    fn set_high(&mut self) -> Result<(), Self::Error>;
}
```

It doesn't specify an error type.
It's very possible that it will return some error on other boards, even if my board returns ```Infallible```.
However, I won't know the specific type of error.
That depends on the board I'm using.
The compiler won't know if the generic parameters return the same error.
To get that information, I would need to couple the implementation to a specific vendor's board.

One option might be a naive implementation of error handling using matches.
In this case I return a ```str``` error as the error type.
If I have a debugger attached or I can print the error to a serial line, that will help diagnose the problem.
But it still doesn't feel right.
The match blocks feel a little verbose.

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

I could try chaining the blocks together with ```and_then``` calls, but each error is a different type.
Remember that the error type is defined in the ```OutputPin``` trait for the embedded_hal crate.
Even if two output pins are exactly the same, they are still distinct types.
I can't chain the errors with ```and_then``` because that relies on specific types.

## Is The Abstraction Right
One code smell about the error handling using ```match``` is that it relies on strings.
I suspect, what I'm missing is a concrete error type.
More than that, I'm missing a level of indirection that allws me to unify the output pin error types.
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
It should have two properties.
The first should clue me in to which pin failed.
Second, I should have a nice error message I can send to a serial port or view in a debugger.

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

This is my first shot at the better error reporting.
One common error strategy is to wrap a given error in another error.
However, I don't know what a specific board will return as an error.
That's why I don't use that pattern here.

### A Type Wrapper
Now I need to wrap the raw generic parameters.
This will deal with the different returned error types.
I create a ```PWMPin``` type, which wraps an output pin.
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

What I did not do is expose this to my library's users.
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

From a library user's pespective, the interface is largely unchanged.
They still pass in their output pins as before.
I silently wrap them.

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

What does change are the return types for the ```PWM5947``` implementation.
The remaining methods fall into line and the error handling looks much tighter.
I return a meaningful error from the device.
The only down-side is that I lose any board specific information.
Of course, this is only the first round of cleanup and nothing is set in stone.
We might be able to preserve that information.
But for now, the error handling is where I think it should be.

