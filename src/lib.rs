//! A [`piet`] backend using [`raqote`].
//!
//! Raqote is a pure-Rust 2D graphics API for writing ARGB images.
//!
//! Raqote's built-in text rendering is not powerful enough to support Piet's text API, so text
//! rendering is provided by [`piet_cosmic_text`] instead.

use std::borrow::Cow;

use image::RaqoteImage;
use piet::{
    FixedGradient, Image, IntoBrush, RenderContext,
    kurbo::{self, Affine, Rect},
};
use piet_cosmic_text::cosmic_text::{self, SwashCache};
use raqote::{
    DrawOptions, DrawTarget, Gradient, Mask, SolidSource, Source, Spread, StrokeStyle, Transform,
    Winding,
};
use tinyvec::tiny_vec;

use crate::image::AsImage;

mod convert;
mod image;
mod text;

use tinyvec::TinyVec;

pub use raqote;

pub struct RaqoteRenderContext<'dt, 'cache, B = Vec<u32>> {
    dt: &'dt mut DrawTarget<B>,
    cache: &'cache mut Cache,
    states: TinyVec<[ContextState; 1]>,
}

impl<'dt, 'cache, B> RaqoteRenderContext<'dt, 'cache, B> {
    pub fn new(dt: &'dt mut DrawTarget<B>, cache: &'cache mut Cache) -> Self {
        RaqoteRenderContext {
            dt,
            cache,
            states: tiny_vec![[ContextState; 1] => ContextState::default()],
        }
    }
}

#[derive(Clone)]
pub struct Brush(BrushInner);

#[derive(Clone)]
enum BrushInner {
    Solid(SolidSource),
    LinearGradient(Gradient, Spread, Transform),
    RadialGradient(Gradient, Spread, Transform),
}

impl Brush {
    fn into_source<'a>(self) -> Source<'a> {
        match self.0 {
            BrushInner::Solid(solid_source) => Source::Solid(solid_source),
            BrushInner::LinearGradient(gradient, spread, transform) => {
                Source::LinearGradient(gradient, spread, transform)
            }
            BrushInner::RadialGradient(gradient, spread, transform) => {
                Source::RadialGradient(gradient, spread, transform)
            }
        }
    }
}

#[derive(Default)]
pub struct Cache {
    text: piet_cosmic_text::Text,

    glyph_cache: Option<SwashCache>,
}

impl Cache {
    pub fn new() -> Self {
        Cache {
            text: piet_cosmic_text::Text::new(),
            glyph_cache: None,
        }
    }
}

impl<B> RenderContext for RaqoteRenderContext<'_, '_, B>
where
    B: AsRef<[u32]> + AsMut<[u32]>,
{
    type Brush = Brush;
    type Image = RaqoteImage;
    type Text = piet_cosmic_text::Text;
    type TextLayout = piet_cosmic_text::TextLayout;

    fn status(&mut self) -> Result<(), piet::Error> {
        Ok(())
    }

    fn clear(&mut self, region: impl Into<Option<Rect>>, color: piet::Color) {
        let region = region.into().unwrap_or_else(|| {
            Rect::new(0.0, 0.0, self.dt.width().into(), self.dt.height().into())
        });

        let (x, y) = (region.origin().x, region.origin().y);
        let (width, height) = (region.size().width, region.size().height);
        let source = Source::Solid(convert::to_color(color).into());
        self.dt.fill_rect(
            x as f32,
            y as f32,
            width as f32,
            height as f32,
            &source,
            &DrawOptions::new(),
        );
    }

    fn solid_brush(&mut self, color: piet::Color) -> Self::Brush {
        let (r, g, b, a) = color.as_rgba8();
        Brush(BrushInner::Solid(SolidSource::from_unpremultiplied_argb(
            a, r, g, b,
        )))
    }

    fn gradient(
        &mut self,
        gradient: impl Into<piet::FixedGradient>,
    ) -> Result<Self::Brush, piet::Error> {
        let inner = match gradient.into() {
            FixedGradient::Linear(linear) => {
                let start = convert::to_point(linear.start);
                let end = convert::to_point(linear.end);

                let source = Source::new_linear_gradient(
                    Gradient {
                        stops: convert::to_stops(linear.stops),
                    },
                    start,
                    end,
                    Spread::Pad,
                );

                match source {
                    Source::LinearGradient(gradient, spread, transform) => {
                        BrushInner::LinearGradient(gradient, spread, transform)
                    }
                    _ => unreachable!(),
                }
            }
            FixedGradient::Radial(radial) => {
                let center = convert::to_point(radial.center);

                let source = Source::new_radial_gradient(
                    Gradient {
                        stops: convert::to_stops(radial.stops),
                    },
                    center,
                    radial.radius as f32,
                    Spread::Pad,
                );

                match source {
                    Source::RadialGradient(gradient, spread, transform) => {
                        BrushInner::RadialGradient(gradient, spread, transform)
                    }
                    _ => unreachable!(),
                }
            }
        };

        Ok(Brush(inner))
    }

    fn fill(&mut self, shape: impl kurbo::Shape, brush: &impl IntoBrush<Self>) {
        let brush = brush.make_brush(self, || shape.bounding_box());

        let mut path = convert::to_path(shape);
        path.winding = Winding::NonZero;

        self.dt.fill(
            &path,
            &brush.into_owned().into_source(),
            &DrawOptions::new(),
        );
    }

    fn fill_even_odd(&mut self, shape: impl kurbo::Shape, brush: &impl IntoBrush<Self>) {
        let brush = brush.make_brush(self, || shape.bounding_box());

        let mut path = convert::to_path(shape);
        path.winding = Winding::EvenOdd;

        self.dt.fill(
            &path,
            &brush.into_owned().into_source(),
            &DrawOptions::new(),
        );
    }

    fn clip(&mut self, shape: impl kurbo::Shape) {
        let path = convert::to_path(shape);
        self.dt.push_clip(&path);
    }

    fn stroke(&mut self, shape: impl kurbo::Shape, brush: &impl IntoBrush<Self>, width: f64) {
        let brush = brush.make_brush(self, || shape.bounding_box());
        let path = convert::to_path(shape);
        let source = brush.into_owned().into_source();
        let style = StrokeStyle {
            width: width as f32,
            ..Default::default()
        };
        self.dt.stroke(&path, &source, &style, &DrawOptions::new());
    }

    fn stroke_styled(
        &mut self,
        shape: impl kurbo::Shape,
        brush: &impl IntoBrush<Self>,
        width: f64,
        style: &piet::StrokeStyle,
    ) {
        let brush = brush.make_brush(self, || shape.bounding_box());
        let path = convert::to_path(shape);
        let source = brush.into_owned().into_source();
        let style = convert::to_stroke_style(width, style);
        self.dt.stroke(&path, &source, &style, &DrawOptions::new());
    }

    fn text(&mut self) -> &mut Self::Text {
        &mut self.cache.text
    }

    fn draw_text(&mut self, layout: &Self::TextLayout, pos: impl Into<kurbo::Point>) {
        let pos = pos.into();
        let mut line_processor = piet_cosmic_text::LineProcessor::new();

        for run in layout.layout_runs() {
            for glyph in run.glyphs {
                let color = glyph.color_opt.unwrap_or_else(|| {
                    let piet_color = piet::util::DEFAULT_TEXT_COLOR;
                    let (r, g, b, a) = piet_color.as_rgba8();
                    cosmic_text::Color::rgba(r, g, b, a)
                });
                line_processor.handle_glyph(glyph, run.line_y, color);

                self.draw_glyph(pos, glyph, run.line_y);
            }
        }
    }

    fn save(&mut self) -> Result<(), piet::Error> {
        let state = self.states.last().unwrap();

        self.states.push(ContextState {
            transform: state.transform,
            clip: state.clip.clone(),
        });

        Ok(())
    }

    fn restore(&mut self) -> Result<(), piet::Error> {
        if self.states.len() == 1 {
            return Err(piet::Error::StackUnbalance);
        }

        self.states.pop();

        Ok(())
    }

    fn finish(&mut self) -> Result<(), piet::Error> {
        Ok(())
    }

    fn transform(&mut self, transform: Affine) {
        self.states.last_mut().unwrap().transform = transform;
    }

    fn current_transform(&self) -> Affine {
        self.states.last().unwrap().transform
    }

    fn make_image(
        &mut self,
        width: usize,
        height: usize,
        buf: &[u8],
        format: piet::ImageFormat,
    ) -> Result<Self::Image, piet::Error> {
        let data: Vec<u32> = match format {
            piet::ImageFormat::RgbaPremul => buf
                .chunks_exact(4)
                .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
                .collect(),
            piet::ImageFormat::RgbaSeparate => buf
                .chunks_exact(4)
                .map(|chunk| {
                    let [mut r, mut g, mut b, a]: [u8; 4] = chunk.try_into().unwrap();

                    let premultiply = |source: u8| {
                        (f32::from(source) * f32::from(a) / f32::from(u8::MAX)).round() as u8
                    };

                    r = premultiply(r);
                    g = premultiply(g);
                    b = premultiply(b);

                    u32::from_le_bytes([r, g, b, a])
                })
                .collect(),
            piet::ImageFormat::Rgb => buf
                .chunks_exact(3)
                .map(|chunk| {
                    let [r, g, b]: [u8; 3] = chunk.try_into().unwrap();
                    u32::from_le_bytes([r, g, b, 0xff])
                })
                .collect(),
            piet::ImageFormat::Grayscale => buf
                .iter()
                .map(|v| u32::from_le_bytes([*v, *v, *v, 0xff]))
                .collect(),
            _ => return Err(piet::Error::NotSupported),
        };

        Ok(RaqoteImage::new(width as i32, height as i32, data))
    }

    fn draw_image(
        &mut self,
        image: &Self::Image,
        dst_rect: impl Into<Rect>,
        interp: piet::InterpolationMode,
    ) {
        let bounds = kurbo::Rect::from_origin_size((0.0, 0.0), image.size());
        self.draw_image_area(image, bounds, dst_rect, interp);
    }

    fn draw_image_area(
        &mut self,
        image: &Self::Image,
        src_rect: impl Into<Rect>,
        dst_rect: impl Into<Rect>,
        _interp: piet::InterpolationMode,
    ) {
        let src_image = RaqoteImage::from_region(image, src_rect);
        let dst_rect = dst_rect.into();

        self.dt.draw_image_with_size_at(
            dst_rect.width() as f32,
            dst_rect.height() as f32,
            dst_rect.x0 as f32,
            dst_rect.y0 as f32,
            &src_image.as_image(),
            &DrawOptions::new(),
        );
    }

    fn capture_image_area(
        &mut self,
        src_rect: impl Into<Rect>,
    ) -> Result<Self::Image, piet::Error> {
        Ok(RaqoteImage::from_region(&*self.dt, src_rect))
    }

    fn blurred_rect(&mut self, rect: Rect, blur_radius: f64, brush: &impl IntoBrush<Self>) {
        let size = piet::util::size_for_blurred_rect(rect, blur_radius);
        let width = size.width as i32;
        let height = size.height as i32;
        if width == 0 || height == 0 {
            return;
        }

        let mut mask = Mask {
            width,
            height,
            data: vec![0; width as usize * height as usize],
        };
        let blurred_rect =
            piet::util::compute_blurred_rect(rect, blur_radius, width as usize, &mut mask.data);

        let source = brush
            .make_brush(self, || blurred_rect)
            .into_owned()
            .into_source();

        self.dt.mask(&source, rect.x0 as i32, rect.y0 as i32, &mask);
    }
}

struct ContextState {
    transform: kurbo::Affine,
    clip: Option<raqote::Path>,
}

impl Default for ContextState {
    fn default() -> Self {
        ContextState {
            transform: kurbo::Affine::IDENTITY,
            clip: None,
        }
    }
}

impl<B> IntoBrush<RaqoteRenderContext<'_, '_, B>> for Brush
where
    B: AsRef<[u32]> + AsMut<[u32]>,
{
    fn make_brush<'a>(
        &'a self,
        _: &mut RaqoteRenderContext<B>,
        _: impl FnOnce() -> piet::kurbo::Rect,
    ) -> std::borrow::Cow<'a, Brush> {
        Cow::Borrowed(self)
    }
}
