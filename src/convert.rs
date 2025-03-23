//! Conversions from [`piet`] types to [`raqote`] types.

use piet::kurbo::{self, PathEl};
use piet_cosmic_text::cosmic_text;
use raqote::PathBuilder;

pub fn to_point(point: kurbo::Point) -> raqote::Point {
    raqote::Point::new(point.x as f32, point.y as f32)
}

pub fn to_color(color: piet::Color) -> raqote::Color {
    let (r, g, b, a) = color.as_rgba8();
    raqote::Color::new(a, r, g, b)
}

pub fn to_stops(stops: impl piet::GradientStops) -> Vec<raqote::GradientStop> {
    stops
        .to_vec()
        .into_iter()
        .map(|stop| raqote::GradientStop {
            position: stop.pos,
            color: to_color(stop.color),
        })
        .collect()
}

pub fn to_stroke_style(width: f64, style: &piet::StrokeStyle) -> raqote::StrokeStyle {
    raqote::StrokeStyle {
        width: width as f32,
        cap: match style.line_cap {
            piet::LineCap::Butt => raqote::LineCap::Butt,
            piet::LineCap::Round => raqote::LineCap::Round,
            piet::LineCap::Square => raqote::LineCap::Square,
        },
        join: match style.line_join {
            piet::LineJoin::Miter { .. } => raqote::LineJoin::Miter,
            piet::LineJoin::Round => raqote::LineJoin::Round,
            piet::LineJoin::Bevel => raqote::LineJoin::Bevel,
        },
        miter_limit: if let piet::LineJoin::Miter { limit } = style.line_join {
            limit as f32
        } else {
            raqote::StrokeStyle::default().miter_limit
        },
        dash_array: style.dash_pattern.iter().map(|e| *e as f32).collect(),
        dash_offset: style.dash_offset as f32,
    }
}

pub fn to_path(shape: impl kurbo::Shape) -> raqote::Path {
    let mut builder = PathBuilder::new();

    for element in shape.path_elements(1e-3) {
        match element {
            PathEl::MoveTo(p) => {
                builder.move_to(p.x as f32, p.y as f32);
            }
            PathEl::LineTo(p) => {
                builder.line_to(p.x as f32, p.y as f32);
            }
            PathEl::QuadTo(p1, p2) => {
                builder.quad_to(p1.x as f32, p1.y as f32, p2.x as f32, p2.y as f32);
            }
            PathEl::CurveTo(p1, p2, p3) => {
                builder.cubic_to(
                    p1.x as f32,
                    p1.y as f32,
                    p2.x as f32,
                    p2.y as f32,
                    p3.x as f32,
                    p3.y as f32,
                );
            }
            PathEl::ClosePath => builder.close(),
        }
    }

    builder.finish()
}

pub fn text_command(cmd: cosmic_text::Command) -> kurbo::PathEl {
    macro_rules! cvt_vector {
        ($v:expr) => {{
            let [x, y]: [f32; 2] = $v.into();
            kurbo::Point::new(x as f64, y as f64)
        }};
    }

    match cmd {
        piet_cosmic_text::cosmic_text::Command::Close => kurbo::PathEl::ClosePath,
        piet_cosmic_text::cosmic_text::Command::MoveTo(p) => kurbo::PathEl::MoveTo(cvt_vector!(p)),
        piet_cosmic_text::cosmic_text::Command::LineTo(p) => kurbo::PathEl::LineTo(cvt_vector!(p)),
        piet_cosmic_text::cosmic_text::Command::QuadTo(p1, p2) => {
            kurbo::PathEl::QuadTo(cvt_vector!(p1), cvt_vector!(p2))
        }
        piet_cosmic_text::cosmic_text::Command::CurveTo(p1, p2, p3) => {
            kurbo::PathEl::CurveTo(cvt_vector!(p1), cvt_vector!(p2), cvt_vector!(p3))
        }
    }
}
