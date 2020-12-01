# PET 2001 Emulator on STM32F103 BluePill

## What?

This is a PET 2001 emulator running on a BluePill dev board. Written in Rust (and some inline assembly).
Video is displayed via composite video. The emulated disk drive uses GD25Q64CSIG spi flash.

It's totally a just for fun personal project.

The emulation is probably not perfect and definitely pretty slow. As you can see in the video creating a nice replica of the original machine also was a non-goal. My original goal was to test the video output - everything else was just to have something interesting to display.

Here is a video of it in action: (click to open it on YouTube)

[![Video](http://img.youtube.com/vi/QxffeJKS6hY/0.jpg)](http://www.youtube.com/watch?v=QxffeJKS6hY "Video")

If you want to build the code you have to build it with the release profile - it won't work in debug mode.

## Pins used

- PB8 Video to video via 680 ohms resitor 
- PB9 Video-Sync to video via 330 ohms resistor

- PA0 Keyboard Select 1
- PA1 Keyboard Select 2
- PA2 Keyboard Select 3
- PA3 Keyboard Select 4
- PA4 Keyboard Select 5
- PA5 Keyboard Select 6
- PA6 Keyboard Select 7
- PA7 Keyboard Select 8

- PB0  Keyboard Input 1
- PB1  Keyboard Input 2
- PB10 Keyboard Input 3
- PB11 Keyboard Input 4
- PA8  Keyboard Input 5
- PA10 Keyboard Input 6
- PA11 Keyboard Input 7
- PA15 Keyboard Input 8
- PB3  Keyboard Input 9
- PB4  Keyboard Shift

SPI FLASH
- SCK = PB13 = 6
- MISO = PB14 = 2
- MOSI = PB15 = 5
- GND = 4
- VCC 3.3V = 8
- HOLD -> VCC -> 7
- WP -> VCC -> 3
- CS = PA9 = 1

## Video

I used these sites to learn about this:
- http://www.batsocks.co.uk/readme/video_timing.htm
- https://github.com/eprive/STM32Lvideo
- https://github.com/eprive/STM32Lvideo/wiki

PB9 is controlled by TIM4

TIM1 is used as a "shock absorber" as outlined in https://github.com/abelykh0/VGA-demo-on-bluepill

## Disk Emulation

This uses the SPI flash to save / load programs. In bluepet/src/bin/add_files.rs is some code to add a PET program.
Unfortunately there isn't enough free memory to use the full 4k blocks of the flash so I "waste" 3/4 of each block unfortunately.

## Keyboard

It's a usual matrix keyboard with 8 select lines and 9 data lines plus the shift keys (both connected to the same pin). I totally mixed up wires during soldering so the keyboard mapping in the code is somewhat odd.

## Code Organization

There are three crates
- mos6502 - the CPU emulation
- pet - other hardware emulation plus some trait definitions for the hardware emulation
- bluepet - the actual emulator using the other crates - implements the traits from the pet crate to use the given hardware
