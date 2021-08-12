use crate::config;
use arrayvec::ArrayVec;
use printpdf::{Mm, Pt};
use std::collections::HashMap;
use std::io;

mod error;
mod util;

pub use error::PdfError;

#[allow(dead_code)]
pub enum Size {
    Mm(f64, f64),
    Px(f64, f64),
    Pt(f64, f64),
}

impl Size {
    fn to_mm(self, dpi: u16) -> (Mm, Mm) {
        let px_to_mm = |x| Mm::from(Pt(util::px_to_pt(x, dpi)));

        match self {
            Size::Mm(x, y) => (Mm(x), Mm(y)),
            Size::Pt(x, y) => (Pt(x).into(), Pt(y).into()),
            Size::Px(x, y) => (px_to_mm(x), px_to_mm(y)),
        }
    }
}

/// a rectangle inside the pdf document
/// (bottom-left)
#[derive(Debug, PartialEq)]
pub struct PdfRect(config::Rectangle<Pt>);

impl PdfRect {
    /// creates an pdf rectangle from a "scalor" rectangle
    fn from(r: config::Rectangle<f64>, size: (Pt, Pt)) -> Self {
        let config::Rectangle {
            mut orig,
            size: r_size,
        } = r;

        // make it left bottom
        orig.y = 1.0 - (orig.y + r_size.y);

        Self(config::Rectangle {
            orig: Self::scale_to_pt(orig, &size),
            size: Self::scale_to_pt(r_size, &size),
        })
    }

    /// scales the point from the given size
    fn scale_to_pt(
        config::Point { x, y }: config::Point<f64>,
        size: &(Pt, Pt),
    ) -> config::Point<Pt> {
        config::Point {
            x: size.0 * x,
            y: size.1 * y,
        }
    }

    /// constructs all the points for drawing inside printpdf
    fn to_points(&self) -> Vec<(printpdf::Point, bool)> {
        let point = |x, y| (printpdf::Point { x, y }, false);
        let config::Rectangle { orig: o, size: s } = self.0;
        vec![
            point(o.x, o.y),
            point(o.x + s.x, o.y),
            point(o.x + s.x, o.y + s.y),
            point(o.x, o.y + s.y),
        ]
    }
}

struct RtFont<'a> {
    inner: rusttype::Font<'a>,
    scale: rusttype::Scale,
    line_height: Pt,
}

impl<'a> RtFont<'a> {
    fn from_rt(font: rusttype::Font<'a>) -> Self {
        let v_metrics = font.v_metrics_unscaled();
        let line_height = (v_metrics.ascent - v_metrics.descent/*+ v_metrics.line_gap*/)
            / font.units_per_em() as f32;
        Self {
            inner: font,
            scale: rusttype::Scale::uniform(line_height),
            line_height: Pt(line_height as f64),
        }
    }

    fn text_width<'b, I>(&'b self, font_size: f32, text: I) -> impl Iterator<Item = f32> + 'b
    where
        I: Iterator<Item = char> + 'b,
    {
        let rt_font = &self.inner;
        rt_font
            .glyphs_for(text)
            .scan(None, move |last: &mut Option<rusttype::GlyphId>, g| {
                let kerning = if let Some(last) = last {
                    rt_font.pair_kerning(self.scale, *last, g.id())
                } else {
                    0.0
                };

                *last = Some(g.id());
                // gets the width of the glyph
                let width = self.get_width(font_size, g.id());

                // the total width is the glyph itself and also the space since the last glyph
                Some(kerning + width)
            })
    }

    fn get_width<G: rusttype::IntoGlyphId>(&self, font_size: f32, glyph: G) -> f32 {
        self.inner
            .glyph(glyph)
            .scaled(self.scale)
            .h_metrics()
            .advance_width
            * font_size
    }
}

pub struct TextArgs<'a> {
    pub area: PdfRect,
    pub font_size: f64,
    pub font: &'a str,
    pub orientation: &'a config::Orientation,
}

pub struct Document {
    /// a map to the index of a font
    /// fontname -> index
    font_map: HashMap<String, usize>,
    /// all fonts loaded as the printpdf format
    pdf_fonts: Vec<printpdf::IndirectFontRef>,
    /// all fonts loaded as the rusttype format
    rt_fonts: Vec<RtFont<'static>>,
    /// fontconfig for finding the font paths
    font_config: fontconfig::Fontconfig,

    /// the printpdf document
    inner_doc: printpdf::PdfDocumentReference,
    size: (Mm, Mm),
    drawing_area: PdfRect,
    dpi: u16,
}

// redefine for easier use in this module
type Result<T, E = PdfError> = std::result::Result<T, E>;

impl Document {
    pub fn new<S: Into<String>>(
        name: S,
        size: Size,
        drawing_area: config::Rectangle<f64>,
        dpi: u16,
    ) -> Result<Self> {
        let size = size.to_mm(dpi);
        let pt_size = (size.0.into(), size.1.into());
        Ok(Self {
            size,
            drawing_area: dbg!(PdfRect::from(drawing_area, pt_size)),
            dpi,
            font_map: Default::default(),
            pdf_fonts: vec![],
            rt_fonts: vec![],
            font_config: fontconfig::Fontconfig::new().ok_or(PdfError::NoFontConfig)?,
            inner_doc: printpdf::PdfDocument::empty(name),
        })
    }

    pub fn save<W: io::Write>(self, to: W) -> Result<(), printpdf::Error> {
        let mut buf_writer = io::BufWriter::new(to);
        self.inner_doc.save(&mut buf_writer)
    }

    /// add a new page to the document, all future operation will be done
    /// on that new page
    pub fn new_page<S: Into<String>>(&'_ mut self, name: S) -> Page<'_> {
        let (page, layer) = self.inner_doc.add_page(self.size.0, self.size.1, name);
        let page = self.inner_doc.get_page(page);
        let layer = page.get_layer(layer);

        let page = Page {
            doc: self,
            page,
            layer,
        };

        #[cfg(debug_assertions)]
        page.draw_rect(&page.doc.drawing_area, None, Some(Page::DBG_COLOR));

        page
    }

    pub fn scale_pdf_rect(&self, area: config::Rectangle<f64>) -> PdfRect {
        let draw_area_size = self.drawing_area.0.size.into();
        let mut tmp = PdfRect::from(area, draw_area_size);
        tmp.0.orig += self.drawing_area.0.orig;
        tmp
    }

    fn fonts(&self, name: &str) -> (&printpdf::IndirectFontRef, &RtFont<'static>) {
        let index = *self.font_map.get(name).unwrap_or(&0);
        (&self.pdf_fonts[index], &self.rt_fonts[index])
    }

    fn maybe_load_font(&mut self, name: &str) -> Result<()> {
        // it is already loaded
        if self.font_map.contains_key(name) {
            return Ok(());
        }

        // find the font path
        let path = self
            .font_config
            .find(name, None)
            .ok_or_else(|| PdfError::FontNotFound(String::from(name)))?
            .path;

        // read the font with printpdf and rusttype
        let (pdf_font, rt_font) = std::fs::read(&path)
            .ok()
            .and_then(|data| {
                Some((
                    self.inner_doc.add_external_font(&data[..]).ok()?,
                    rusttype::Font::try_from_vec(data)?,
                ))
            })
            // give the path to the font if it couldn't be loaded
            .ok_or_else(|| PdfError::FontNotLoaded(path.to_string_lossy().into()))?;

        // add the fonts to the map and lists
        self.rt_fonts.push(RtFont::from_rt(rt_font));
        self.pdf_fonts.push(pdf_font);
        let index = self.rt_fonts.len() - 1;
        self.font_map.insert(String::from(name), index);

        Ok(())
    }
}

struct LineData {
    end_index: usize,
    width: f32,
}

struct PositionArgs<'a> {
    text_args: &'a TextArgs<'a>,
    line_height: f64,
    lines: &'a ArrayVec<LineData, 64>,
}

impl<'a> PositionArgs<'a> {
    fn new(args: &'a TextArgs<'a>, lines: &'a ArrayVec<LineData, 64>, font: &RtFont<'_>) -> Self {
        Self {
            lines,
            // TODO: needs to be changed
            line_height: dbg!(font.line_height.0 * args.font_size),
            text_args: args,
        }
    }
}

pub struct Page<'a> {
    pub doc: &'a mut Document,
    page: printpdf::PdfPageReference,
    layer: printpdf::PdfLayerReference,
}

impl<'a> Page<'a> {
    pub fn new_layer<S: Into<String>>(&mut self, name: S) {
        self.layer = self.page.add_layer(name);
    }

    const DBG_COLOR: printpdf::Color = printpdf::Color::Rgb(printpdf::Rgb {
        r: 1.0,
        g: 0.0,
        b: 1.0,
        icc_profile: None,
    });

    pub fn draw_rect(
        &self,
        rect: &PdfRect,
        fill_color: Option<printpdf::Color>,
        stroke_color: Option<printpdf::Color>,
    ) {
        let layer = &self.layer;
        let line = printpdf::Line {
            points: rect.to_points(),
            is_closed: true,
            has_fill: fill_color.is_some(),
            has_stroke: stroke_color.is_some(),
            is_clipping_path: false,
        };

        // set the color
        fill_color.map(|c| layer.set_fill_color(c));
        stroke_color.map(|c| layer.set_outline_color(c));

        // and draw it
        layer.add_shape(line)
    }

    pub fn draw_text(&mut self, args: &TextArgs<'_>, text: String) -> Result<Pt> {
        // draw the box outlines in debug mode
        #[cfg(debug_assertions)]
        self.draw_rect(&args.area, None, Some(Self::DBG_COLOR));

        // get the fonts
        self.doc.maybe_load_font(args.font)?;
        let (pdf_font, rt_font) = self.doc.fonts(args.font);

        // reassign for readability
        let width = args.area.0.size.x.0;
        let font_size = args.font_size;
        let whitespace_width = rt_font.get_width(font_size as f32, ' ');
        let layer = &self.layer;

        // PANICS: content with more than 64 lines should be a sin
        // TODO: maybe use Vec for better memory usage
        let beginnings: ArrayVec<_, 64> =
            Self::get_lines(rt_font, &text, font_size as f32, width, whitespace_width).collect();
        let mut pos_args = PositionArgs::new(args, &beginnings, rt_font);

        let mut i = 0;
        let mut start = 0; // start at index 0, duh

        for line in beginnings.iter() {
            // prepare line
            let end = line.end_index;
            let pos = self.get_position(i, &mut pos_args);

            dbg!(line.width, start, end, &text[start..end]);
            layer.use_text(&text[start..end], font_size, pos.x, pos.y, &pdf_font);

            start = end;
            i += 1;
        }

        Ok(Pt((i + 1) as f64 * font_size))
    }

    fn get_lines<'b>(
        font: &'b RtFont<'b>,
        text: &'b str,
        font_size: f32,
        width: f64,
        whitespace_width: f32,
    ) -> impl Iterator<Item = LineData> + 'b {
        eprintln!("max width of the line: {}", width);

        // TODO: maybe support other chars
        text.split_ascii_whitespace()
            .map(move |word| {
                Some((
                    // the start index of the word
                    util::get_index_of(word, text),
                    // the width of the word
                    // TODO: add the width of the pending whitespace
                    font.text_width(font_size, word.chars()).sum(),
                ))
            })
            .chain(std::iter::once(None)) // marks the end of the text
            .filter_map(is_line_end(width as f32, whitespace_width, text.len()))
    }

    fn get_position(&self, line_idx: usize, args: &mut PositionArgs<'_>) -> config::Point<Mm> {
        let orientation = dbg!(args.text_args.orientation);
        let area = &args.text_args.area.0;
        let size = dbg!(area.size);

        use config::HorOrientation as Hor;
        use config::VertOrientation as Vert;

        let y = match orientation.vertical {
            Vert::Top => size.y.0 - (line_idx + 1) as f64 * args.line_height,
            Vert::Middle => size.y.0 / 2.0 - line_idx as f64 * args.line_height,
            Vert::Bottom => (args.lines.len() - (line_idx + 1)) as f64 * args.line_height,
        };

        let width = args.lines[line_idx].width;
        let x = match orientation.horizontal {
            Hor::Left => 0.0,
            Hor::Middle => (size.x.0 - width as f64) / 2.0,
            Hor::Right => size.x.0 - width as f64,
        };

        let pos = config::Point { x: Pt(x), y: Pt(y) } + dbg!(area.orig);
        dbg!(pos);
        pos.map(|pt| pt.into())
    }
}

fn is_line_end(
    max_width: f32,
    whitespace_width: f32,
    str_len: usize,
) -> impl FnMut(Option<(usize, f32)>) -> Option<LineData> {
    let mut p_sum = 0.0;

    move |o| {
        if let Some((i, w)) = o {
            p_sum += w;

            if p_sum > max_width {
                let tmp = Some(LineData {
                    end_index: i,
                    width: p_sum,
                });

                p_sum = w;
                tmp
            } else {
                // add the whitespace width if this word is still on the line
                p_sum += whitespace_width;
                None
            }
        } else {
            Some(LineData {
                end_index: str_len,
                width: p_sum,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{Point, Rectangle};
    use printpdf::{Mm, Pt};

    fn equal_within_error(left: f64, right: f64) {
        assert!((left - right).abs() < 0.001)
    }

    const DPI: u16 = 96;

    #[test]
    fn size_mm_to_mm() {
        let left = 24.3;
        let right = 12.2;

        assert_eq!(
            super::Size::Mm(left, right).to_mm(DPI),
            (Mm(left), Mm(right))
        );
    }

    #[test]
    fn size_pt_to_mm() {
        let left = 24.3;
        let right = 12.2;
        let (result_x, result_y) = super::Size::Pt(left, right).to_mm(DPI);
        let expected_x = 8.5725;
        let expected_y = 4.30388;

        equal_within_error(result_x.0, expected_x);
        equal_within_error(result_y.0, expected_y);
    }

    #[test]
    fn size_px_to_mm() {
        let left = 1920.0;
        let right = 1080.0;
        let (result_x, result_y) = super::Size::Px(left, right).to_mm(DPI);
        let expected_x = 508.0;
        let expected_y = 285.75;

        equal_within_error(result_x.0, expected_x);
        equal_within_error(result_y.0, expected_y);
    }

    const RECT_SIZE: (Pt, Pt) = (Pt(100.0), Pt(100.0));
    #[test]
    fn rect_upperleft_origin() {
        let rect = Rectangle {
            orig: Point { x: 0.0, y: 0.0 },
            size: Point { x: 1.0, y: 1.0 },
        };

        let expected = Rectangle {
            orig: Point {
                x: Pt(0.0),
                y: Pt(0.0),
            },
            size: Point {
                x: RECT_SIZE.0,
                y: RECT_SIZE.1,
            },
        };

        assert_eq!(
            super::PdfRect::from(rect, RECT_SIZE),
            super::PdfRect(expected)
        );
    }
}
