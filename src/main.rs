use std::fs::File;

use crate::{
    config::Config,
    drawing::{pdf_maker::PdfMaker, DrawError, Drawer},
};

mod config;
mod drawing;
mod parser;
mod util;

fn main() -> Result<(), DrawError> {
    //println!("Hello, world!");
    let file = std::env::args().nth(1).expect("usage: slidmk <file>");
    println!("file read from: {}", file);
    let slides = parser::parse_file(file);
    let config = Config::default();

    let mut pdf = PdfMaker::with_config(&config).expect("couldn't get the pdfmaker");
    let slides = slides.filter(|s| match s.kind == "Style" {
        // TODO: load the style sheet
        true => false,
        false => true,
    });
    pdf.create_slides(slides, &config)
        .expect("Counldn't not create the slides do to");
    let file = File::create("output.pdf").expect("couldn't open file");
    pdf.write(file)
}
