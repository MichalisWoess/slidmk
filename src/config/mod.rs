use crate::drawing::error::DrawError;
use std::collections::HashMap;
pub type StyleMap = HashMap<String, SlideTemplate>;

mod de_se;
mod primitives;

pub use primitives::*;

#[derive(Debug)]
pub struct Decoration {
    pub area: Rectangle<f64>,
    pub color_idx: usize,
}

#[derive(Debug)]
pub struct ContentTemplate {
    pub area: Rectangle<f64>,
    // TODO: add line spacing
    pub font_size: f32,
    pub orientation: Orientation,
}

#[derive(Debug)]
pub struct SlideTemplate {
    /// a decoration for the slides
    /// draws a simple rectangle at the given position(item0) with the color from the index
    pub decorations: Vec<Decoration>,
    /// an area were content can appear
    pub content: Vec<ContentTemplate>,
}

#[derive(Debug)]
pub struct PresentStyle {
    pub colors: Vec<Color>,
    pub font: String,
    line_spacing: f64,
}

#[derive(Debug)]
pub struct Config<'a> {
    pub style: PresentStyle,
    pub margin: Rectangle<f64>,
    pub slide_styles: StyleMap,
    pub doc_name: &'a str,
}

impl<'a> Config<'a> {
    pub fn get_color(&self, idx: usize) -> Result<Color, DrawError> {
        self.style
            .colors
            .get(idx)
            .ok_or(DrawError::NoColor(idx))
            .map(|c| *c)
    }
}

impl<'a> Default for Config<'a> {
    fn default() -> Self {
        let header_orientation = Orientation {
            vertical: VertOrientation::Bottom,
            horizontal: HorOrientation::Middle,
        };

        Self {
            doc_name: "default",
            style: PresentStyle {
                colors: vec![
                    Color::new(0.0, 0.0, 0.0),
                    Color::new(1.0, 0.0, 0.0),
                    Color::new(0.0, 1.0, 1.0),
                ],
                font: String::from("monospace"),
                line_spacing: 1.0,
            },
            margin: Rectangle {
                orig: Point { x: 0.05, y: 0.05 },
                size: Point { x: 0.9, y: 0.9 },
            },
            slide_styles: crate::map! {
                "Title" => SlideTemplate {
                    decorations: vec![],
                    content: vec![
                        ContentTemplate {
                            area: Rectangle {
                                orig: Point{x: 0.0,y: 0.0},
                                size: Point{x: 1.0,y: 0.8} },
                            font_size: 36.0,
                            orientation: header_orientation.clone(),
                        },
                        ContentTemplate {
                            area: Rectangle {
                                orig: Point{x: 0.0,y: 0.8},
                                size: Point{x: 1.0,y: 0.2} },
                            font_size: 18.0,
                            orientation: Orientation::default(),
                        },
                    ],
                },

                "Head_Cont" => SlideTemplate {
                    decorations: vec![],
                    content: vec![
                        ContentTemplate {
                            area: Rectangle {
                                orig: Point{x: 0.0,y: 0.0},
                                size: Point{x: 1.0,y: 0.3},
                            },
                            font_size: 24.0,
                            orientation: header_orientation.clone(),
                        },
                        ContentTemplate {
                            area: Rectangle {
                                orig: Point{x: 0.0,y: 0.3},
                                size: Point{x: 1.0,y: 0.7},
                            },
                            font_size: 18.0,
                            orientation: Orientation::default(),
                        },
                    ],
                },

                "Vert_Split" => SlideTemplate {
                    decorations: vec![],
                    content: vec![
                        ContentTemplate {
                            area: Rectangle {
                                orig: Point{x: 0.0,y: 0.0},
                                size: Point{x: 0.5,y: 0.3},
                            },
                            font_size: 24.0,
                            orientation: header_orientation.clone(),
                        },
                        ContentTemplate {
                            area: Rectangle {
                                orig: Point{x: 0.0,y: 0.3},
                                size: Point{x: 0.5,y: 0.7},
                            },
                            font_size: 18.0,
                            orientation: Orientation::default(),
                        },
                        ContentTemplate {
                            area: Rectangle {
                                orig: Point{x: 0.5,y: 0.0},
                                size: Point{x: 0.5,y: 0.3},
                            },
                            font_size: 24.0,
                            orientation: header_orientation,
                        },
                        ContentTemplate {
                            area: Rectangle {
                                orig: Point{x: 0.5,y: 0.3},
                                size: Point{x: 0.5,y: 0.7},
                            },
                            font_size: 18.0,
                            orientation: Orientation::default(),
                        },
                    ],
                },
                "Two_Hor" => SlideTemplate {
                    decorations: vec![],
                    content: vec![
                        ContentTemplate {
                            area: Rectangle {
                                orig: Point{x: 0.0,y: 0.0},
                                size: Point{x: 1.0,y: 0.5},
                            },
                            font_size: 20.0,
                            orientation: Orientation::default(),
                        },
                        ContentTemplate {
                            area: Rectangle {
                                orig: Point{x: 0.0,y: 0.5},
                                size: Point{x: 1.0,y: 0.5},
                            },
                            font_size: 20.0,
                            orientation: Orientation::default(),
                        },
                    ],
                },
            },
        }
    }
}
