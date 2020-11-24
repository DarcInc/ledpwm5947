# Learning to Write Rust
Let me start by admitting I don't know Rust.
Or, rather, I've read about Rust, I've played with Rust, but I haven't bled with Rust.
You don't really know a language until you've had it in your hands long enough to find the sharp corners, cut yourself, and bleed a little.
The only way you get a language into your hands is to start and finish a project with it.
The more skin in the game, the more you have to push on the language, the more those share corners come out.
Eventually, you'll get cut and maybe learn something in the process.

A work deliverable would be ideal.
Unfortunately, I don't code much anymore at work.
It's a shame.
I really like to build things in code.
It's like getting paid to solve puzzles all day long.
So I have to make something up.
In this case I went to the darker corners of Rust, where embedded Rust lives.
So I grabbed [a board off ST Microelectronic's site](https://www.st.com/content/st_com/en/products/evaluation-tools/product-evaluation-tools/mcu-mpu-eval-tools/stm32-mcu-mpu-eval-tools/stm32-nucleo-boards/nucleo-l432kc.html) and got after it.

## The Problem in a Nutshell
I have these [RGB LEDs](https://www.adafruit.com/product/1451) and a [PWM controller](https://www.adafruit.com/product/1429) to make blinky lights.
I originally bought it to make some light-up fairy wings for my daughter's Halloween costume, years ago.
Let's start with something simple.
Let's make the LED flash red, then green, then blue, then white.
I loosely copied the logic from [Adafruit's GitHub repo](https://github.com/adafruit/Adafruit_TLC5947) as a starter.

My goals are to get the PWM working, isolate the code into a library, and make it a first-class Rust library.
The first part will just introduce me to the embedded Rust toolchain, compiler, and configuration for the board.
The second and third objectives are a little more interesting.
The second is to get my feet wet with libraries, crates, and Rust tooling, like Cargo.
I want to produce libraries, so I need to understand library management.
The third is to actually learn what good Rust looks like.
On one level it is fixing the issues identified by the linter.
On another level it's about learning Rust well and internalizing that knowledge. 

### Getting Started
I'll admit I was overconfident.
The original fairy wing project on the [Arduino Nano](https://store.arduino.cc/usa/arduino-nano) went smoothly.
That's largely because the Arduino IDE and tooling does a lot of the heavy lifting for you.
It installs a tool chain and correctly loads the program, given your board and I/O configuration.
If I wanted to use C or C++, the tools from ST or other vendors probably have much the same experience.
Choosing Rust was going to require some more work on my part.

I installed the Rust tool chain and embedded tools on my [Pinebook Pro](https://www.pine64.org/pinebook-pro/) but ran into problems.
The compiler could build the source for the target processor, but GDB would not load the binaries on the STM32L432 or start the debugger.
I began by checking and re-checking the configuration settings.
I was certain the problem was in the configuration settings that indicate where the binary should be loaded in memory.
I spent a couple of evenings digging through spec sheets, making certain the memory configuration matched ST's documentation.
After dozens of attempts with different settings, I realized I didn't have the correct [OpenOCD](http://openocd.org) configuration.
OpenOCD is the implementation of the on-chip debugger library that allows me to interact with the debugger built in to the evaluation board.
Do I really need the debugger to work?

### You DO Need a Debugger
Let's say you're writing code on the desktop and there's a problem.
If the debugger doesn't work (or no one taught you how to properly use one), you can always just print values.
The `print` statements aren't as good as stepping through with a debugger, but we've all done it.
Printing values to the terminal isn't (generally) possible on embedded hardware.
You can sometimes write the output to a serial port, but that itself requires your code to basically work.
You need a debugger.

Traditionally, debugging embedded hardware meant hooking a piece of additional hardware to the microcontroller.
Sometimes you have to solder additional pins on the board, or maybe hand-craft a cable to mate the hardware debugger with the microcontroller.
The evaluation boards from ST have a built-in debugger.
A working OpenOCD tool allows me to access that all important debugger.
Fortunately, there's [Cargo Generate](https://github.com/ashleygwilliams/cargo-generate) and [a great working example](https://github.com/reneherrero/nucleo-l432kc-quickstart).
Unfortunately, it still didn't work.
I thought about trying it on another computer.

The GDB, OpenOCD and Rust toolchain works on my Mac Mini.
I can even debug directly from Visual Studio code without using command-line GDB.
I want to get down to the problem of porting over the PWM driver.
I'll leave debugging the possible quirks of embedded Rust on the PineBook Pro aside for a future date.
My PineBook Pro is fun the same way English sports-cars are fun.
Not everything might work correctly (ARM Linux isn't as developed as x86/x86_64 Linux) but its a tinkerer's delight.
The Mac is like a mid-market luxury sedan I just take to the shop.
I imagine a Windows or x86_64 Linux desktop would work just as well.

With [VS Code](https://code.visualstudio.com), The Rust, Rust-Analzyer, and Cortex-Debug extensions I'm good to go.
I can set a breakpoint, start running, and stop to inspect what's going.
It's almost as easy as desktop development.
Except the code is compiled for embedded arm and running on the micro-controller.
I still find myself firing up command-line GDB every now and then.
Mostly because of muscle memory.
Because I'm old.

### Have Spare LEDs
It's been a while since I've worked on an embedded project.
I'd forgotten about a little trick I learned.
As I said above, ```printf``` is worthless.
You can, however, attach blinky lights.
If you attach an LED to a pin, you can see when it turns on.
Even if it's a rapid series of pulses, you can see it's flickering and something is happening.
Turning on a light is far from perfect, but if writing code that runs on servers and desktops is a battle, embedded is a knife-fight in a back alley.
You take whatever you get.

## My Rig
The micro-controller sits on a bread-board for easy access to the pins.
The specific form-factor I chose pushes into the breadboard, straddling the middle divider.
The 5v output and ground pins are connected to the 5947, along with the pins D4, D5, D6, and D7.
I have a RGB LED hooked up to pins 0, 1, and 2 of the 5947.
I found the little prototyping and testing board I'd made for the fairy wing project.
It's a gaggle of wires, resistors, and LEDs that only its maker could love.

### It Works
I did a quick check to make sure it was wired up (roughly) correctly by first using an Arduino Nano.
The test code ran fine and the PWM worked as expected.
All I need to do is replace the Arduino Nano with the STM32L432 and it should work.

### It Doesn't Work
Next, I plugged into the STM32L432.
I started up the translated code.
Sadly, the code did not work.
I started the debugging process by walking through the code in my head.
This is a skill worth developing, but it requires you to think on the same level as the processor.
When I say it 'looks right', I mean I'm basically walking it through line by line.
I get are random flashes of color when I move the board instead of red, green, and blue, 

I noticed that when I push or pull on the wiring I get different flashes.
A general rule of thumb is that if you jiggle a wire and something hapens, you have a loose wire.
I try pushing and pulling on different wires.
It could be the breadboard, maybe the jumper wire, or maybe something came loose.
I replace the breadboard on the theory my jumper wires aren't making good contact, but there's no change.

I next check the connector I soldered on the 5947 to see if it came loose.
Maybe the board cracked?
I've cracked boards in the past.
The problem often shows up like a loose wire as the traces on the board make and break contact.
I push and pull on the connector, checking to make sure it's solid.
I inspect the board with a magnifying glass for cracks.
Nope, the board is fine.

Did I somehow manage to kill the controller?
It's possible I somehow fried the controller.
That's when I remembered the LED trick.
I hook up the LEDs and realize the data pin D7 never lights up.
Why is D7 important?
Because D7 is controlled by a different register from the other pins.
This is the kind of thing the Arduino hides from you.

I now go through the code to set a PIN for IO.
It should be on and set as an output pin.
I review the spec sheets and try a few different things.
I'm I can't seem to turn that register on.
That's something to debug later.
I move the data pin to D3, which is on the same register as D4, D5 and D6.

### It Works Again
Victory.
The controller cycles the PWM from red, to green, to blue and finally to white.
I can start the program in a debugger and step through the lines.

What do I know at this point?
My compiler configuration is correct.
The on-chip debugger works.
While I had some issues with D7, I can turn on pins and turn off pins.
The communication to the PWM is fine.
Basically, I've got my environment configured.
I have left a few things to come back to, but those aren't what I really want to work on right now.

## Isolate The Bad Library
With a working test environment, I can pull the 5947 guts out into this library.
My original code to "just get it working," is throw-away.
The whole point was to get the core logic.
I want to be able to manipulate the PWM driver in an (ideally) micro-processor agnostic fashion.
While I don't have a RISC-V chip to test with, for example, it would be nice if it also worked with that architecture.
I'd like my code dependencies to be as abstract and as high on the layer cake of dependencies as possible.
I need to remove all the STM32L432 Nucleo specific items.

I also don't want a slap-dash library.
There are some people that tell you it doesn't matter if the code's good.
The only thing that matters is that it works.
Don't listen to those people.
They're idiots.
Especially don't listen to them if the code runs for years on end on some device like a pacemaker.
Good code is not always the difference between life and death, but it might be the difference between leaving work at five or six on a regular basis instead of midnigh.
The first implementation works, but it's a little bit of an eye-sore.

I also want a library that is good Rust.
At the time I write this, I suspect it's not good Rust.
What I haven't done is to look at my library and enough other Rust code to see what that's the case.
Like I said before, I don't know Rust.
I feel like I left a little blood on the table, getting to this point, and I've learned a little.
Now the real job starts, understanding why my code is bad.