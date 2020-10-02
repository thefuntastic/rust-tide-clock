#[cfg(target_arch = "arm")]
use crate::ssd1305::Ssd1305Controller;
use crate::tides::{TideExtremeGraphData, TideModel, TideModelWindow};
use crate::{font::Font5, maths};
use chrono::{DateTime, Local, Utc};
use image::{Rgb, RgbImage};
use std::{cmp::max, path::Path};

const PIXEL_WHITE: Rgb<u8> = Rgb([255_u8, 255_u8, 255_u8]);
const PIXEL_BLACK: Rgb<u8> = Rgb([0_u8, 0_u8, 0_u8]);
const SCREEN_WIDTH: u32 = 128;
const SCREEN_HEIGHT: u32 = 32;

// 0 | 1 | 0
// 1 | 1 | 1
// 1 | 1 | 1
const FLOOD_FILL_MASK: [u32; 9] = [0, 1, 0, 1, 1, 1, 1, 1, 1];

pub struct Position {
    x: u32,
    y: u32,
}

pub struct Bounds {
    w: u32,
    h: u32,
}

pub trait RenderDevice {
    fn render(&mut self, buffer: &RgbImage);
}

#[cfg(target_arch = "arm")]
impl RenderDevice for Ssd1305Controller {
    fn render(&mut self, buffer: &RgbImage) {
        self.clear();
        //println!("clearing image");
        for (x, y, pixel) in buffer.enumerate_pixels() {
            let x = x as usize;
            let y = y as usize;
            let c: u8 = pixel[0]; //Access red channel
            self.set_pixel(x, y, c);
        }
        self.display();
    }
}

pub struct ImageWriter {}

impl RenderDevice for ImageWriter {
    fn render(&mut self, buffer: &RgbImage) {
        let out = Path::new("resources/display.bmp");
        buffer.save(out).unwrap();
    }
}

pub trait Painter {
    fn paint(&self, buffer: &mut RgbImage, now: DateTime<Utc>);
}

pub struct TextField<'a> {
    text: String,
    pos: Position,
    bounds: Bounds,
    font: &'a Font5,
}

impl TextField<'_> {
    pub fn new(text: String, font: &Font5, x: u32, y: u32) -> TextField {
        let mut tf = TextField {
            text: String::new(),
            font,
            pos: Position { x, y },
            bounds: Bounds { w: 0, h: 0 },
        };

        tf.set_text(text); //Update bounds

        tf
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;

        let mut width = 0;
        let mut height: u32 = 0;
        for c in self.text.chars() {
            if let Some(si) = self.font.faces.get(&c) {
                width += si.width() + 1;
                height = max(height, si.height());
            }
        }

        self.bounds = Bounds {
            w: width,
            h: height,
        };
    }
}

impl Painter for TextField<'_> {
    fn paint(&self, buffer: &mut RgbImage, _now: DateTime<Utc>) {
        //bounds check

        let mut caret = 0;
        for c in self.text.chars() {
            //println!("{}", c);

            if let Some(si) = self.font.faces.get(&c) {
                image::imageops::overlay(buffer, si, self.pos.x + caret, self.pos.y);
                caret += si.width() + 1;
            }
        }
    }
}

pub struct GraphCanvas<'a> {
    pos: Position,
    bounds: Bounds,
    data: &'a TideModelWindow<'a>,
    font: &'a Font5,
}

impl GraphCanvas<'_> {
    pub fn new<'a>(
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        data: &'a TideModelWindow<'a>,
        font: &'a Font5,
    ) -> GraphCanvas<'a> {
        GraphCanvas {
            pos: Position { x, y },
            bounds: Bounds { w, h },
            data,
            font,
        }
    }
}

impl Painter for GraphCanvas<'_> {
    fn paint(&self, buffer: &mut RgbImage, now: DateTime<Utc>) {
        for col in 0..self.bounds.w {
            for row in 0..self.bounds.h {
                let raw = calculate_pixel(
                    &self.data.normalised_heights,
                    &self.bounds,
                    col as i32,
                    row as i32,
                );

                let pixel = match raw {
                    0 => PIXEL_BLACK,
                    1 => PIXEL_WHITE,
                    _ => PIXEL_BLACK,
                };

                buffer.put_pixel(self.pos.x + col, self.pos.y + row, pixel);
            }
        }

        //Race condition: labels depend on drawn wave data to draw descenders
        for data_point in self.data.extremes().iter() {
            let data_index_in_window = self.data.get_extreme_index_in_window(data_point.index());
            let label = ExtremeLabel::new(self.font, data_point, data_index_in_window, &self.pos);
            label.paint(buffer, now);
        }

        // Draw play head
        let mut current_index: u32 = 0;
        //let now = Utc.ymd(2020, 9, 14).and_hms(9, 39, 00);
        if let Some(index) = TideModel::find_time_index(self.data.dates, now) {
            let x = self.pos.x + index;
            current_index = index; //record result

            //Draw play head
            if x < SCREEN_WIDTH {
                for y in self.pos.y..SCREEN_HEIGHT {
                    let px = match y % 2 {
                        0 => PIXEL_WHITE,
                        1 => PIXEL_BLACK,
                        _ => PIXEL_BLACK,
                    };

                    buffer.put_pixel(self.pos.x + index, y, px);
                }
            }
        }

        // Flood fill erase to remove waves in the past
        for col in 0..current_index {
            for row in 0..self.bounds.h {
                let x = col as i32;
                let y = row as i32; //Invert y axis

                let kernel: [u32; 9] = [
                    calculate_pixel(&self.data.normalised_heights, &self.bounds, x - 1, y - 1),
                    calculate_pixel(&self.data.normalised_heights, &self.bounds, x, y - 1),
                    calculate_pixel(&self.data.normalised_heights, &self.bounds, x + 1, y - 1),
                    calculate_pixel(&self.data.normalised_heights, &self.bounds, x - 1, y),
                    calculate_pixel(&self.data.normalised_heights, &self.bounds, x, y),
                    calculate_pixel(&self.data.normalised_heights, &self.bounds, x + 1, y),
                    calculate_pixel(&self.data.normalised_heights, &self.bounds, x - 1, y + 1),
                    calculate_pixel(&self.data.normalised_heights, &self.bounds, x, y + 1),
                    calculate_pixel(&self.data.normalised_heights, &self.bounds, x + 1, y + 1),
                ];

                if should_erase(kernel, &FLOOD_FILL_MASK) {
                    let screen_x = self.pos.x + col;
                    let screen_y = self.pos.y + row;

                    if screen_x >= SCREEN_WIDTH || screen_y >= SCREEN_HEIGHT {
                        continue;
                    }

                    buffer.put_pixel(screen_x, screen_y, PIXEL_BLACK);
                }
            }
        }
    }
}

fn calculate_pixel(normalized_heights: &[f32], bounds: &Bounds, x: i32, y: i32) -> u32 {
    if x < 0 || x >= normalized_heights.len() as i32 {
        return 1;
    }

    let height = match normalized_heights.get(x as usize) {
        Some(h) => h.to_owned(),
        None => 0_f32,
    };

    let px_height = maths::lerp(height, 0, bounds.h as i32) as u32;
    let y_pos = bounds.h - px_height;

    if y >= y_pos as i32 {
        return 1;
    }

    0
}

fn should_erase(kernel: [u32; 9], mask: &[u32; 9]) -> bool {
    // kernel        mask         result (fails)
    // 0 | 0 | 0     0 | 1 | 0    0 | 0 | 0
    // 1 | 1 | 1  ?  1 | 1 | 1  = 1 | 1 | 1
    // 1 | 1 | 1     1 | 1 | 1    1 | 1 | 1

    for i in 0..9 {
        let result = kernel[i] & mask[i];

        if result ^ mask[i] == 1 {
            return false;
        }
    }
    true
}

pub struct ExtremeLabel<'a> {
    text_field: TextField<'a>,
}

impl ExtremeLabel<'_> {
    pub fn new<'a>(
        font: &'a Font5,
        data: &'a TideExtremeGraphData,
        data_index: u32,
        canvas_pos: &Position,
    ) -> ExtremeLabel<'a> {
        let pos = Position {
            x: canvas_pos.x + data_index,
            y: 0,
        };

        let local_tz = Local::now().timezone();
        let local_dt = data.date().with_timezone(&local_tz);

        ExtremeLabel {
            text_field: TextField::new(local_dt.format("%H:%M").to_string(), font, pos.x, pos.y),
        }
    }
}

impl Painter for ExtremeLabel<'_> {
    fn paint(&self, buffer: &mut RgbImage, now: DateTime<Utc>) {
        self.text_field.paint(buffer, now);

        let baseline = self.text_field.pos.y + self.text_field.bounds.h + 2_u32;

        //Draw underline
        for i in 0..(self.text_field.bounds.w - 1) {
            let x = self.text_field.pos.x + i;
            let y = baseline;

            if x < SCREEN_WIDTH && y < SCREEN_HEIGHT {
                buffer.put_pixel(x, y, PIXEL_WHITE);
            }
        }

        // Draw descenders
        let x = self.text_field.pos.x;
        let mut highest: u32 = 0;
        if x < SCREEN_WIDTH {
            //Find highest wave pixel
            for y in (0..SCREEN_HEIGHT).rev() {
                let px = buffer.get_pixel(x, y);

                //When we find pixel whose r channel is 0, bail and set as highest.
                if px[0] == 0 {
                    highest = y;
                    break;
                }
            }

            //Draw from a baseline to highest one above highest for 1px gap. This may be eq or above the base line, in which case nothing gets drawn
            for y in baseline..(highest - 1) {
                buffer.put_pixel(x, y, PIXEL_WHITE);
            }
        }
    }
}

pub struct WaterMark<'a> {
    pos: Position,
    bounds: Bounds,
    tide_model: &'a TideModel,
}

impl WaterMark<'_> {
    pub fn new(x: u32, y: u32, w: u32, h: u32, tide_model: &TideModel) -> WaterMark {
        WaterMark {
            pos: Position { x, y },
            bounds: Bounds { w, h },
            tide_model,
        }
    }
}

impl Painter for WaterMark<'_> {
    fn paint(&self, buffer: &mut RgbImage, now: DateTime<Utc>) {
        //Draw upper + lower notch
        buffer.put_pixel(self.pos.x, self.pos.y, PIXEL_WHITE);
        buffer.put_pixel(self.pos.x, self.pos.y + self.bounds.h - 1, PIXEL_WHITE);

        //Draw bar
        for row in 0..self.bounds.h {
            buffer.put_pixel(self.pos.x + 1_u32, self.pos.y + row, PIXEL_WHITE);
        }

        //Draw water mark
        let t = self.tide_model.get_current_norm_height(now);
        let y_pos: u32 = maths::lerp(
            t,
            (self.pos.y + self.bounds.h - 1) as i32,
            self.pos.y as i32,
        ) as u32;

        let mark_y = y_pos;
        let mark_x: u32 = match mark_y == self.pos.y || mark_y == self.pos.y + self.bounds.h - 1 {
            true => self.pos.x - 1, //Offset by 1 pixel if at upper or lower notch
            false => self.pos.x,
        };

        buffer.put_pixel(mark_x, mark_y, PIXEL_WHITE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_erase() {
        let kernel = [0, 1, 1, 1, 1, 1, 1, 1, 1];

        assert_eq!(should_erase(kernel, &FLOOD_FILL_MASK), true);

        let kernel = [1, 1, 0, 1, 1, 1, 1, 1, 1];
        assert_eq!(should_erase(kernel, &FLOOD_FILL_MASK), true);
    }

    #[test]
    fn test_calculate_pixel() {
        // 0 0 0 1
        // 0 0 1 1
        // 0 1 1 1
        // 1 1 1 1
        let normalized_heights: [f32; 4] = [0.25, 0.5, 0.75, 1.0];
        let bounds = Bounds { w: 4, h: 4 };

        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 0, 0), 0);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 1, 0), 0);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 2, 0), 0);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 3, 0), 1);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 0, 1), 0);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 1, 1), 0);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 2, 1), 1);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 3, 1), 1);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 0, 2), 0);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 1, 2), 1);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 2, 2), 1);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 3, 2), 1);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 0, 3), 1);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 1, 3), 1);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 2, 3), 1);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 3, 3), 1);

        // 0 0 0 0 0 1
        // 0 0 0 0 0 1
        // 0 0 0 0 1 1
        // 0 0 0 0 1 1
        // 0 0 0 1 1 1
        // 0 0 0 1 1 1
        // 0 0 1 1 1 1
        // 0 0 1 1 1 1
        // 0 1 1 1 1 1
        // 0 1 1 1 1 1
        let normalized_heights: [f32; 6] = [0_f32, 0.2, 0.4, 0.6, 0.8, 1_f32];
        let bounds = Bounds { w: 6, h: 10 };

        // x 0 0 0 0 0 1
        // x 0 0 0 0 0 1
        // x 0 0 0 0 1 1
        // x 0 0 0 0 1 1
        // x 0 0 0 1 1 1
        // x 0 0 0 1 1 1
        // x 0 0 1 1 1 1
        // x 0 0 1 1 1 1
        //|x 0 1|1 1 1 1
        //|x 0 1|1 1 1 1
        //|x x x|x x x x
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, -1, 8), 1);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 0, 8), 0);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 1, 8), 1);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, -1, 9), 1);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 0, 9), 0);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 1, 9), 1);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, -1, 10), 1);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 0, 10), 1);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 1, 10), 1);

        // x x x x x x x x
        // 0 0 0 0|0 1 x|x
        // 0 0 0 0|0 1 x|x
        // 0 0 0 0|1 1 x|x
        // 0 0 0 0 1 1 x x
        // 0 0 0 1 1 1 x x
        // 0 0 0 1 1 1 x x
        // 0 0 1 1 1 1 x x
        // 0 0 1 1 1 1 x x
        // 0 1 1 1 1 1 x x
        // 0 1 1 1 1 1 x x
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 4, 0), 0);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 5, 0), 1);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 6, 0), 1);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 4, 1), 0);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 5, 1), 1);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 6, 1), 1);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 4, 2), 1);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 5, 2), 1);
        assert_eq!(calculate_pixel(&normalized_heights, &bounds, 6, 2), 1);
    }
}
