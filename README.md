# open-modern-multimeter
Open Modern Multimeter - virtual visualization for remote multimeter physical data measurements. Modern data loggers extension.

## Motivations
When you create your own precision measuring instruments for physical values, sometimes you need to see the current values ​​on your PC/laptop/Mac.

Those exciting moments when you can see the expected data sets with important values ​​on your desktop give you the power to express and say “wow, it works!” :-)

Overall, it is a clever tool to visualize data values ​​for learning and having fun with measurements in electronics and measurement!

The idea is trivial. When you have a device that records real-time measurement values ​​to USB or RS-232 or RS-485 port, this tool will help you display these values ​​in 7-segment display style on your computer screen.

The convenience is that you don’t have to look at the measuring instrument - the measurement results are visible in this program.


### What you can see after running this program?

#### Standard multimeter view with value
![Example of DC voltage measurements from quite precission instrument](assets/github.com--bieli--open-modern-multimeter--screenshot--001.png)

#### Extended multimeter view with value and linear chart
![Example of DC voltage measurements with histogram statistics chart](assets/github.com--bieli--open-modern-multimeter--screenshot--002.png)

#### Extended multimeter view with value and histogram statistics chart
![Example of DC voltage measurements with linear chart](assets/github.com--bieli--open-modern-multimeter--screenshot--003.png)



## How to run

### RUST language ecosystem installation

Official page about `rustup` with instructions is [here](https://www.rust-lang.org/tools/install)

### From exists Makefile
```bash
$ make run

# or

$ cargo run /dev/ttyUSB0 115200 1 VDC 3_3 l
```

### From built release
```bash
$ cargo build --release

$ ./target/release/open-modern-multimeter --help
```


## How to create virtual terminal for testing/emulating measures/values (on Linux)
```bash
$ sudo socat PTY,link=/dev/ttyS10 PTY,link=/dev/ttyS111
$ sudo chmod a+wrx /dev/ttyS111
$ cargo run /dev/ttyS111 115200 1 VDC 3_3
```
If you would like to use real null-modem emulator running on Linux kernel level (with real timers), please use this [Linux driver for nullmodem](https://github.com/pitti98/nullmodem).

The general concept for emulating terminal in UNIX/Linux/*BSD operating systems is called [Pseudoterminal](https://en.wikipedia.org/wiki/Pseudoterminal).


## Features and arguments list in program

```bash
$ ./target/release/open-modern-multimeter --help
Open Modern Multimeter 
Reads values from an external multimeter via a serial port and displays measurement values in
real-time in a UI

USAGE:
    open-modern-multimeter <port> <baud> <channel_no> <unit> <window_position> [ARGS]

ARGS:
    <port>               The device path to the serial port
    <baud>               The baud rate for communication
    <channel_no>         The channel number to display
    <unit>               The unit of measurement
    <window_position>    Setting up program window position on the screen <x_pos>_<y_pos>, where
                         x_pos and y_pos are in range {1..4} (i.e. 3_3 in the middle of the
                         screen)
    <enable_chart>       Enable dynamic charts (h: histogram, l: linear) on bottom side of
                         measurement screen. [default: ]
    <color>              Color of the display values: r for red, g for green, b for blue
                         (default color is red if not specified) [default: r]

OPTIONS:
    -h, --help    Print help information
```

