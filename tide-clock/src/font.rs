use image::{GenericImageView, RgbImage};
use std::collections::HashMap;
use std::path::Path;

pub struct Font5 {
    pub faces: HashMap<char, RgbImage>,
}

impl Font5 {
    pub fn new() -> Font5 {
        let p = Path::new("resources/Font-5px.png");

        let img = image::open(p).unwrap().to_rgb();

        let mut faces = HashMap::new();

        //So in theory it's not necessary to call img.view().to_image() and instead to have a memory effecient reference to the original image. aka img.view()
        //However rust starts complaining about the subimage outliving the lifetime of the original image.
        //And quite simply I don't know enough rust to fix that
        //Something something lifetime annotations... but that's as far as I got.
        //However the error won't look so lush, it will say something about expectect struct got reference
        //https://squidarth.com/rc/rust/2018/05/31/rust-borrowing-and-ownership.html

        faces.insert(' ', img.view(124, 0, 2, 5).to_image());
        faces.insert('_', img.view(124, 0, 1, 5).to_image());
        faces.insert('1', img.view(0, 0, 1, 5).to_image());
        faces.insert('2', img.view(2, 0, 3, 5).to_image());
        faces.insert('3', img.view(6, 0, 3, 5).to_image());
        faces.insert('4', img.view(10, 0, 3, 5).to_image());
        faces.insert('5', img.view(14, 0, 3, 5).to_image());
        faces.insert('6', img.view(18, 0, 3, 5).to_image());
        faces.insert('7', img.view(22, 0, 3, 5).to_image());
        faces.insert('8', img.view(26, 0, 3, 5).to_image());
        faces.insert('9', img.view(30, 0, 3, 5).to_image());
        faces.insert('0', img.view(34, 0, 3, 5).to_image());
        faces.insert(':', img.view(38, 0, 1, 5).to_image());
        faces.insert('.', img.view(40, 0, 1, 5).to_image());
        faces.insert('m', img.view(42, 0, 5, 5).to_image());
        faces.insert('f', img.view(48, 0, 2, 5).to_image());
        faces.insert('t', img.view(51, 0, 2, 5).to_image());
        faces.insert('!', img.view(54, 0, 1, 5).to_image());
        faces.insert('?', img.view(56, 0, 3, 5).to_image());
        faces.insert('A', img.view(0, 6, 3, 5).to_image());
        faces.insert('B', img.view(4, 6, 3, 5).to_image());
        faces.insert('C', img.view(8, 6, 3, 5).to_image());
        faces.insert('D', img.view(12, 6, 3, 5).to_image());
        faces.insert('E', img.view(16, 6, 3, 5).to_image());
        faces.insert('F', img.view(20, 6, 3, 5).to_image());
        faces.insert('G', img.view(24, 6, 3, 5).to_image());
        faces.insert('H', img.view(28, 6, 3, 5).to_image());
        faces.insert('I', img.view(32, 6, 1, 5).to_image());
        faces.insert('J', img.view(34, 6, 3, 5).to_image());
        faces.insert('K', img.view(38, 6, 3, 5).to_image());
        faces.insert('L', img.view(42, 6, 3, 5).to_image());
        faces.insert('M', img.view(46, 6, 5, 5).to_image());
        faces.insert('N', img.view(52, 6, 3, 5).to_image());
        faces.insert('O', img.view(56, 6, 3, 5).to_image());
        faces.insert('P', img.view(60, 6, 3, 5).to_image());
        faces.insert('Q', img.view(64, 6, 4, 5).to_image());
        faces.insert('R', img.view(69, 6, 3, 5).to_image());
        faces.insert('S', img.view(73, 6, 3, 5).to_image());
        faces.insert('T', img.view(77, 6, 3, 5).to_image());
        faces.insert('U', img.view(81, 6, 3, 5).to_image());
        faces.insert('V', img.view(85, 6, 3, 5).to_image());
        faces.insert('W', img.view(89, 6, 5, 5).to_image());
        faces.insert('X', img.view(95, 6, 3, 5).to_image());
        faces.insert('Y', img.view(99, 6, 3, 5).to_image());
        faces.insert('Z', img.view(103, 6, 3, 5).to_image());
        faces.insert('(', img.view(0, 12, 6, 6).to_image());
        faces.insert(')', img.view(7, 12, 6, 6).to_image());
        faces.insert('[', img.view(14, 12, 6, 6).to_image());
        faces.insert(']', img.view(21, 12, 6, 6).to_image());

        Font5 { faces }
    }
}

pub fn init() -> Font5 {
    Font5::new()
}
