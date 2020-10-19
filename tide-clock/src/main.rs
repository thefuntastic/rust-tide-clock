use chrono::{DateTime, Local, Utc};
use font::Font5;
use image::RgbImage;
use std::{error::Error, thread, time};
use tides::{TideModel, TideModelWindow};
mod display;
mod font;
mod maths;
mod tides;
use display::{GraphCanvas, Painter, RenderDevice, TextField, WaterMark};

// When cross-compiling, use display emulation. When compiling
// for target hardware, use the actual hardware.
// This should be feature flag instead, but was unaware of the language feature at the time of implementation
#[cfg(not(target_arch = "arm"))]
use display::ImageWriter;
#[cfg(target_arch = "arm")]
mod ssd1305;

const MAX_RETRIES: i32 = 3;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Hello, world!");

    let tide_data = tides::TideResponse::new();
    let mut tide_model = TideModel::new(tide_data);

    let range = tide_model.get_date_range().unwrap();
    println!("Found date range on disk: {:?} at {:?}", range, Utc::now());

    let font = font::init();

    //Because we're using RenderDevice to hold our reference (aka Trait Object), we don't know the concrete type. This means
    //we need to use a box pointer
    let mut render_device: Box<dyn RenderDevice> = Box::new(init_render_device());

    let mut backbuffer: RgbImage = RgbImage::new(128, 32);

    render("HELLO TIM!", &font, &mut backbuffer);
    render_device.render(&backbuffer);
    thread::sleep(time::Duration::from_secs(4));

    let mut backbuffer: RgbImage = RgbImage::new(128, 32);

    render("YOU LOOK MAGNIFICENT TODAY", &font, &mut backbuffer);
    render_device.render(&backbuffer);
    thread::sleep(time::Duration::from_secs(5));

    //let p = Path::new("resources/FaceDisp.r6.png");
    //let mut img = image::open(p).unwrap().to_rgb();

    //let mut offset = 0;
    let mut retries = 0;

    loop {
        // Test time logic
        // offset += 1;
        // let duration = Duration::minutes(offset * 16);
        // let now = Local::now().checked_add_signed(duration).unwrap();
        let now = Local::now();

        let (window, is_data_fresh) = tide_model.get_window(now);

        match is_data_fresh {
            tides::DataFreshness::Fresh => {
                retries = 0;

                paint(&mut render_device, &font, &tide_model, &window, now);
            }
            tides::DataFreshness::NeedsUpdate => {
                println!("Data needs update, loading api");
                retries += 1;

                if retries > MAX_RETRIES {
                    panic!(
                        "Could not refresh tide data after 3 attempts. Aborting and shutting down"
                    );
                }

                //Blocking - not quite sure yet what the best paradigm is for async code
                let response = tides::load_tides_from_api()?;

                tide_model = TideModel::new(response);

                //Print confirmation to log
                let range = tide_model.get_date_range().unwrap();
                println!(
                    "Loaded date range: {:?} at {:?}",
                    range,
                    tides::local_to_utc(now)
                );

                let (window, _is_data_fresh) = tide_model.get_window(now);

                paint(&mut render_device, &font, &tide_model, &window, now);
            }
        }

        thread::sleep(time::Duration::from_millis(1000))
    }
}

fn paint(
    render_device: &mut Box<dyn RenderDevice>,
    font: &Font5,
    tide_model: &TideModel,
    tide_window: &TideModelWindow,
    local_time: DateTime<Local>,
) {
    let mut time_text = TextField::new("00:00".to_string(), font, 0, 0);
    let mut high_water_text = TextField::new("0.0m".to_string(), font, 0, 8);
    let mut low_water_text = TextField::new("0.0m".to_string(), font, 0, 27);

    let graph = GraphCanvas::new(21, 10, 107, 22, tide_window, &font);
    let water_mark = WaterMark::new(17, 10, 2, 22, tide_model);

    let mut img: RgbImage = RgbImage::new(128, 32);

    let format = match local_time.timestamp() % 2 {
        0 => "%H:%M",
        1 => "%H_%M", //'_' Will be substituted for 1px space, instead of 2px space as used for words
        _ => "%H:%M",
    };
    time_text.set_text(local_time.format(format).to_string());

    high_water_text.set_text(format!("{:.1}m", tide_window.water_mark().high_water));
    low_water_text.set_text(format!("{:.1}m", tide_window.water_mark().low_water));

    let utc_now = tides::local_to_utc(local_time);

    time_text.paint(&mut img, utc_now);
    high_water_text.paint(&mut img, utc_now);
    low_water_text.paint(&mut img, utc_now);

    water_mark.paint(&mut img, utc_now);
    graph.paint(&mut img, utc_now);

    render_device.render(&img);
}

fn render(text: &str, font: &font::Font5, backbuffer: &mut RgbImage) {
    println!("{}", text);

    let mut width = 0;
    for c in text.chars() {
        if let Some(si) = font.faces.get(&c) {
            width += si.width() + 1;
        }
    }

    let mut caret = 128 / 2 - width / 2;
    for c in text.chars() {
        //println!("{}", c);

        if let Some(si) = font.faces.get(&c) {
            image::imageops::replace(backbuffer, si, caret, 13);
            caret += si.width() + 1;
        }
    }
}

#[cfg(target_arch = "arm")]
fn init_render_device() -> ssd1305::Ssd1305Controller {
    let mut controller = ssd1305::init();
    controller.clear();
    controller.set_pixel(5, 5, 1);
    controller.display();
    controller
}

#[cfg(not(target_arch = "arm"))]
fn init_render_device() -> ImageWriter {
    ImageWriter {}
}
