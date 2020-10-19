use rppal::gpio;
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::thread;
use std::time::Duration;

//BCM pin numbers
const GPIO_DC: u8 = 24;
const GPIO_RST: u8 = 25;
const WIDTH: usize = 128;
const HEIGHT: usize = 32;

pub struct Ssd1305Controller {
    gpio_dc: gpio::OutputPin,
    gpio_rst: gpio::OutputPin,
    spi: Spi,

    buffer: [u8; 512],
}

impl Ssd1305Controller {
    fn command(&mut self, cmd: u8) {
        //println!("{}", cmd);

        &self.gpio_dc.write(gpio::Level::Low);

        let write_buffer = vec![cmd]; //Could make this a look up for less memory pressure

        &self.spi.write(&write_buffer).unwrap();
    }

    pub fn display(&mut self) {
        //write the buffer

        for page in 0..4 {
            self.command(0xB0 + page); //Set page address
            self.command(0x04); //Set low column address
            self.command(0x10); //Set high column address
            self.gpio_dc.write(gpio::Level::High);

            //Typing shennigans
            let page_usize = page as usize;
            let start_index: usize = page_usize * WIDTH;
            let end_index: usize = start_index + WIDTH;

            //Every 8 rows is a represented in a 128 bit buffer
            let page_slice = &self.buffer[start_index..end_index];

            self.spi.write(page_slice).unwrap();
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: u8) {
        if x >= WIDTH || y >= HEIGHT {
            println!("SetColor: Pixel out of bounds x:{} y:{}", x, y);
            return;
        }

        //The 128x32 display is split into 4 strips of 128x8,
        //where each column of 8 is encoded into a single value

        if color > 0 {
            self.buffer[x + (y / 8) * WIDTH] |= 1 << (y % 8);
        } else {
            self.buffer[x + (y / 8) * WIDTH] &= !(1 << (y % 8));
        }
    }

    pub fn clear(&mut self) {
        for i in 0..self.buffer.len() {
            self.buffer[i] = 0;
        }
    }
}

fn setup() -> Ssd1305Controller {
    let gpio = gpio::Gpio::new().unwrap();
    let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 8_000_000, Mode::Mode0).unwrap();

    let gpio_dc = gpio.get(GPIO_DC).unwrap().into_output();
    let gpio_rst = gpio.get(GPIO_RST).unwrap().into_output();

    let buffer = [0x00; 512];

    Ssd1305Controller {
        gpio_dc,
        gpio_rst,
        spi: spi,
        buffer: buffer,
    }
}

pub fn init() -> Ssd1305Controller {
    let mut controller = setup();

    reset(&mut controller);

    controller.command(0xAE); //--turn off oled panel
    controller.command(0x04); //--turn off oled panel
    controller.command(0x10); //--turn off oled panel
    controller.command(0x40); //---set low column address
    controller.command(0x81); //---set high column address
    controller.command(0x80); //--set start line address  Set Mapping RAM Display Start Line (0x00~0x3F)
    controller.command(0xA1); //--set contrast control register
    controller.command(0xA6); // Set SEG Output Current Brightness
    controller.command(0xA8); //--Set SEG/Column Mapping     0xa0×óÓÒ·´ÖÃ 0xa1Õý³£
    controller.command(0x1F); //Set COM/Row Scan Direction   0xc0ÉÏÏÂ·´ÖÃ 0xc8Õý³£
    controller.command(0xC8); //--set normal display
    controller.command(0xD3); //--set multiplex ratio(1 to 64)
    controller.command(0x00); //--1/64 duty
    controller.command(0xD5); //-set display offset	Shift Mapping RAM Counter (0x00~0x3F)
    controller.command(0xF0); //-not offset
    controller.command(0xd8); //--set display clock divide ratio/oscillator frequency
    controller.command(0x05); //--set divide ratio, Set Clock as 100 Frames/Sec
    controller.command(0xD9); //--set pre-charge period
    controller.command(0xC2); //Set Pre-Charge as 15 Clocks & Discharge as 1 Clock
    controller.command(0xDA); //--set com pins hardware configuration
    controller.command(0x12);
    controller.command(0xDB); //--set vcomh
    controller.command(0x08); //Set VCOM Deselect Level
    controller.command(0xAF); //-Set Page Addressing Mode (0x00/0x01/0x02)

    set_pixel(&mut controller.buffer, 0, 0, 1);
    set_pixel(&mut controller.buffer, 127, 0, 1);
    set_pixel(&mut controller.buffer, 0, 31, 1);
    set_pixel(&mut controller.buffer, 127, 31, 1);

    controller.display();

    controller
}

fn reset(controller: &mut Ssd1305Controller) {
    controller.gpio_rst.write(gpio::Level::High);
    thread::sleep(Duration::from_millis(10));
    controller.gpio_rst.write(gpio::Level::Low);
    thread::sleep(Duration::from_millis(10));
    controller.gpio_rst.write(gpio::Level::High);
}

fn set_pixel(buffer: &mut [u8; 512], x: usize, y: usize, color: u8) {
    if x >= WIDTH || y >= HEIGHT {
        println!("SetColor: Pixel out of bounds x:{} y:{}", x, y);
        return;
    }

    //The 128x32 display is split into 4 strips of 128x8,
    //where each column of 8 is encoded into a single value

    if color > 0 {
        buffer[x + (y / 8) * WIDTH] |= 1 << (y % 8);
    } else {
        buffer[x + (y / 8) * WIDTH] &= !(1 << (y % 8));
    }
}
