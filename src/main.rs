use std::fs::File;
use structopt::StructOpt;

use crate::{
    config::Config,
    drawing::{pdf_maker::PdfMaker, DrawError, Drawer},
    parser::Content,
};

mod cli_args;
mod config;
mod drawing;
mod parser;
mod util;

fn get_project_dir() -> directories::ProjectDirs {
    directories::ProjectDirs::from("org", "wetlo", "slidmk")
        .expect("Unknown operating system, couldn't find a good project directory")
}

fn main() -> Result<(), DrawError> {
    let args = cli_args::Opts::from_args();
    let dir = get_project_dir();
    let templates = if args.templates.is_empty() {
        vec![dir.config_dir().join("template.hjson")]
    } else {
        args.templates
    };

    let mut config = Config::from_files(
        templates.as_ref(),
        args.style
            .unwrap_or_else(|| dir.config_dir().join("style.hjson")),
    )
    .unwrap_or_default();

    let slides = parser::parse_file(args.present_file);
    let mut pdf = PdfMaker::with_config(&config).expect("couldn't get the pdfmaker");

    for slide in slides {
        match slide.kind.as_str() {
            "Style" => {
                let path = slide
                    .contents
                    .into_iter()
                    .next()
                    .map(|c| match c {
                        Content::Config(p) => Some(p),
                        _ => None,
                    })
                    .flatten()
                    .expect("expected path to the style sheet");

                config
                    .change_style(path)
                    .expect("Couldn't load style sheet");
            }
            _ => pdf
                .create_slide(slide, &config)
                .expect("Counldn't not create the slides do to"),
        }
    }

    let file = File::create(args.output).expect("couldn't open file");
    pdf.write(file)
}
