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

### Arduino Does a Lot For You
I'll admit I was overconfident.
My original crack at this on the [Arduino Nano](https://store.arduino.cc/usa/arduino-nano) went a lot smoother.
I was trying to get the debugger and programmer working on my [Pinebook Pro](https://www.pine64.org/pinebook-pro/).
I couldn't get code to load or the debugger to start.
I had the memory configuration right, but I didn't have the [OpenOCD](http://openocd.org) configuration right.

On embedded hardware, even a dilletant like me, quickly realizes you need a proper debugger.
The boards from ST are well priced and they have a built-in debugger.
Fortunately, there's [Cargo Generate](https://github.com/ashleygwilliams/cargo-generate) and [a great working example](https://github.com/reneherrero/nucleo-l432kc-quickstart).
But it still didn't work.

But it works on my Mac Mini.
That's fine.
I'll run with that.
My Pinebook Pro is fun the same way English sportscars are fun.
The Mac just runs.

With [VS Code](https://code.visualstudio.com), The Rust, Rust-Analzyer, and Cortex-Debug extensions I'm good to go.
I can set a breakpoint, start running, and stop to inspect what's going.
It's almost as easy as desktop development.
Except the code is compiled for embedded arm and running on the micro-controller.
Although I generally just start gdb.
Because I'm old.

### Have Spare LEDs
It's been a while, so I'd forgotten about a little trick I learned.
When you're trying to talk to hardware on a micro-controller, ```printf``` is worthless.
But you can start to attach blinky lights.
If you attach an LED to a pin, you can see when it turns on.
Even if it's flickering, you can see it's doing something.
Not perfect, but if writing code that runs on servers and desktops is a battle, embedded is a knife-fight in a back alley.
You take whatever you get.

## My Rig
I have the micro-controller on a bread-board.
The 5v output and ground are connected to the 5947, along with the pins D4, D5, D6, and D7.
I have a RGB LED hooked up to pins 0, 1, and 2 of the 5947.
I found the little prototyping and testing board I'd made for the fairy wing project.
It's a gaggle of wires, resistors, and LEDs that only its make could love.

But it doesn't work.
The logic looks right.
But I get random flashes of color when I move the board.

Gotta be a loose wire.
I push and pull.
Maybe it's not making good contact on the breadboard.
I replace the breadboard.
Same deal.
Okay, maybe the connector I soldered on the 5947 came loose?

I push and pull.
It's solid.
No wiggle.
Maybe the board cracked?
Nope, the board is fine.

Bad controller?
That's when I remembered the LED trick.
I hook up the LEDs and realize the data pin never lights up.
It's on D7.
Why is that important?
Because D7 is controlled by a different register from the other pins.
I can't seem to turn that register on.
Something to debug later.
I move the data pin to D3, which is on the same reister as D4, D5 and D6.

Victory.
Red then green then blue and finally white.
It works.

## Isolate The Bad Library
Now that I have a way to test the code, I can pull the 5947 guts out into this library.
My original code to "just get it working," is throw-away.
The whole point was to get the core logic.

There are some people that tell you it doesn't matter if the code's good.
The only thing that matters is that it works.
Don't listen to those people.
They're idiots.
Especially don't listen to them if the code runs for years on end on some device like a pacemaker.
Good code is not always the difference between life and death, but it might be the difference between leaving work at five or six on a regular basis instead of midnigh.

I have a library but it's bad.
I'm not sure it's good Rust.
Like I said before, I don't know Rust.
I feel like I left a little blood on the table, getting to this point, and I've learned a little bit.
But now the real job starts, understanding why my code is bad.