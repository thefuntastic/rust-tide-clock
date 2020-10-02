
# Rust + Raspberry Pi Tide Clock

Source code for a digital tide clock with accurate and location specific tidal readings. This maker project conceived as heartwarming birthday present, you can read more about the build and construction [on my blog](https://thefuntastic.com/blog/rust-tide-clock).

I provide no guarantees about idiomatic usage or correct code conventions, in particular as my Rust exposure was limited at the time of writing. The code is merely provided as a sample for the curious, however there are some bits that may be of interest to wider audience.

In particular `src/ssd1305.rs` is a working reference implementation of communicating to a [Waveshare SSD1305](https://thepihut.com/collections/waveshare/products/128x32-2-23inch-oled-display-hat-for-raspberry-pi) by use of the `rppal` crate. This may be of use to other makers working on similar embedded projects. 

## Usage 

In order to build your own working version, you will need a [worldtides.info](https://www.worldtides.info/developer) API key. Duplicate `resources/Secrets-Template.toml` to `resources/Secrets.toml` and substitute your API key.

To change the location, you can use the [WorldTiles console](https://www.worldtides.info/) to find accurate Lat and Lon values. These can then be populated in `resources/Settings.toml`. By default the project will make an API call approximately every 3 days.

## Cross Platform Development

On Raspberry Pi, assuming you've got the appropriate screen attached, all display logic will be output to the screen via the GPIO pins. On other platforms (e.g. Windows), output will instead be saved to `tide-clock/resources/display.bmp`. Visual Studio Code will hot reload images on change, which allows effective development on other platforms

## Cross Platform Compilation 

If you're building on Raspberry Pi 3, running the project is simply a matter of installing rustup and calling `cargo run`. If you wish to target Raspberry Pi Zero, this [blog post](https://piers.rocks/docker/containers/raspberry/pi/rust/cross/compile/compilation/2018/12/16/rust-compilation-for-raspberry-pi.html) provides a guide to the considerations. The `.gitlab-ci.yml` file provides a working implementation of this on **Gitlab** (this is hosted on **Github** for distribution purposes only). 
 
## Licence

The licence is GNU GPL v3. Any makers and other non-commercial users are encouraged to use the code as they wish. Any commercial usage should contact me directly for a commercial licence.   

---

# Personal Anecdata

The following excerpts from my personal are provided for reference only. They do however contain some additional context that might prove useful.

## Restarting the program on Pi boot

There seem to be multiple ways to do this. I'm using `/etc/rc.local` which is a shell script called on startup. This needs to be executable, which it already is on Raspian Noobs build I'm using 
https://unix.stackexchange.com/questions/473901/execute-script-at-startup

Change to home folder (otherwise you might run into paths issues) and then execute the program
```
cd /home/pi/Projects/TideClock/rust/tide-clock/target/arm-unknown-linux-musleabihf/release/
./tide-clock
```


This is a good resource on how to start the program as a service, and is probably the method I prefer if starting again
http://segfaultsourcery.s3-website.eu-central-1.amazonaws.com/snippets/rust/rust-to-raspi/landing.html


## WaveShare SSD1305
The OLED screen is manufactured by Waveshare, is docs are located here 
https://www.waveshare.com/wiki/2.23inch_OLED_HAT

Given the translation levels, the docs can be a bit confusing. It refers to samples which are buried at the bottom of the page:
https://www.waveshare.com/w/upload/c/c5/2.23inch-OLED-HAT-Code.7z

The samples actually are multiply redundant, and show many different ways of achieve the same result. 
Platforms:
1. Arduino
2. Raspberry Pi 
3. STM32 (another hardware platform)

Obviously we are interested in the Raspberry PI. Here they are further broken down by protocol
1. I2C
2. SPI

The screen doesn't do I2C without being resoldered (at least as far as I understand it). So obviously we're using SPI

There are three language examples
1. bcm2835
2. python 
3. wiringPi

### BCM2835

As far as I understand, bcm is the Broadcom controller for the the Pi's GPIO pins. This 3rd party C library needs to be compiled and built, and then speaks to the controller natively. 

I was dissuaded from this by the fact that the library needs to be compiled.

### Python

This would be the most convenient option, however I can't tell why it doesn't work. Not knowing enough about the Pi and it's environment I've abandoned this

### wiringPi 

WiringPI is an open source C effort that seems to be distributed with Raspbian. This makes it the "defacto", however it seems the maintainer has (recently) [stepped down](http://wiringpi.com/wiringpi-deprecated/) from the library, so it's unclear how long it will stay viable for. 

For reasons I'm not quite sure of, each approach labels the pin numbers differently. A reference can be found here in the Waveshare samples:
`Raspberry Pi/SPI/wiringPi/readme.txt` 
(`rppal` uses BCM numbering)

Otherwise this examples seems to work well and is easy to understand. 

My efforts are centred around porting this example to rust. 

In order update the screen first call make
```
$ make
```
Followed by 
```
$ sudo ./oled
```

If you just try run the sample directly it will error as it was presumably built for a different architecture 

### SPI and GPIO

I'll admit my understanding of this isn't stellar, however I know SPI is a format for sending packed data in an efficient format. However the SSD1305 is mostly sending pixel data over `SPI`, while most of the other controller data is sent directly to GPIO pins, which follow a simple Arduino high/low convention.

## Rust 

### Rppal Crate 

With version 0.10.0 the library changed it's API to per pin access. From what I can understand this brings technical benefits (thread safety etc), however it also includes advanced rust type shenannigans I struggled with.

This real world project based on v0.6.0 was close enough to the patterns I was thinking of to serve as a reference for our project.
https://github.com/KaneTW/pt100-controller/blob/master/src/main.rs

NB it seems v0.9.0 is the latest equivalent with a compatible API to v0.6.0;

### Image Crate 

https://docs.rs/image/0.23.8/image/

Extra usage samples here
https://github.com/ha-shine/rustagram/blob/master/src/rustaops/mod.rs

https://github.com/image-rs/image/blob/master/src/lib.rs