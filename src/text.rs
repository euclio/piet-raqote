// Source code in this file is heavily inspired by piet-tiny-skia, and is licensed under MPL-2.0.
//
// SPDX-License-Identifier: MPL-2.0

use std::slice;

use piet::{
    Color, RenderContext,
    kurbo::{self, Affine, Shape},
};
use piet_cosmic_text::cosmic_text::{self, Command, SwashCache};

use crate::{RaqoteRenderContext, convert};

impl<B> RaqoteRenderContext<'_, '_, B>
where
    B: AsRef<[u32]> + AsMut<[u32]>,
{
    pub(crate) fn draw_glyph(
        &mut self,
        pos: kurbo::Point,
        glyph: &cosmic_text::LayoutGlyph,
        run_y: f32,
    ) {
        let mut glyph_cache = self
            .cache
            .glyph_cache
            .take()
            .unwrap_or_else(SwashCache::new);

        let physical = glyph.physical((0., 0.), 1.0);
        self.cache.text.clone().with_font_system_mut(|system| {
            if let Some(outline) = glyph_cache.get_outline_commands(system, physical.cache_key) {
                let offset = kurbo::Affine::translate((
                    pos.x + physical.x as f64 + physical.cache_key.x_bin.as_float() as f64,
                    pos.y
                        + run_y as f64
                        + physical.y as f64
                        + physical.cache_key.y_bin.as_float() as f64,
                )) * Affine::scale_non_uniform(1.0, -1.0);
                let color = glyph.color_opt.map_or(piet::util::DEFAULT_TEXT_COLOR, |c| {
                    Color::rgba8(c.r(), c.g(), c.b(), c.a())
                });

                // Fill in the outline.
                self.fill_even_odd(
                    TextShape {
                        cmds: outline,
                        offset,
                    },
                    &color,
                );
            } else {
                // Blit the image onto the target.
                let default_color = {
                    let (r, g, b, a) = piet::util::DEFAULT_TEXT_COLOR.as_rgba8();
                    cosmic_text::Color::rgba(r, g, b, a)
                };
                glyph_cache.with_pixels(system, physical.cache_key, default_color, |x, y, clr| {
                    let [r, g, b, a] = [clr.r(), clr.g(), clr.b(), clr.a()];
                    let color = Color::rgba8(r, g, b, a);

                    // Straight-blit the image.
                    self.fill_even_odd(
                        kurbo::Rect::from_origin_size((x as f64, y as f64), (1., 1.)),
                        &color,
                    );
                });
            }
        });

        self.cache.glyph_cache = Some(glyph_cache);
    }
}

pub struct TextShape<'a> {
    pub cmds: &'a [Command],
    pub offset: kurbo::Affine,
}

impl Shape for TextShape<'_> {
    type PathElementsIter<'iter>
        = TextPathElements<'iter>
    where
        Self: 'iter;

    fn path_elements(&self, _tolerance: f64) -> Self::PathElementsIter<'_> {
        TextPathElements {
            inner: self.cmds.iter(),
            offset: self.offset,
        }
    }

    fn area(&self) -> f64 {
        self.to_path(1.0).area()
    }

    fn perimeter(&self, accuracy: f64) -> f64 {
        self.to_path(accuracy).perimeter(accuracy)
    }

    fn winding(&self, pt: kurbo::Point) -> i32 {
        self.to_path(1.0).winding(pt)
    }

    fn bounding_box(&self) -> kurbo::Rect {
        self.to_path(1.0).bounding_box()
    }
}

#[derive(Clone)]
pub struct TextPathElements<'a> {
    inner: slice::Iter<'a, Command>,
    offset: kurbo::Affine,
}

impl TextPathElements<'_> {
    fn leap(&self) -> impl Fn(&Command) -> kurbo::PathEl + use<> {
        let offset = self.offset;
        move |&cmd| offset * convert::text_command(cmd)
    }
}

impl Iterator for TextPathElements<'_> {
    type Item = kurbo::PathEl;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(self.leap())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.inner.nth(n).map(self.leap())
    }

    fn fold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        let m = self.leap();
        self.inner.map(m).fold(init, f)
    }
}
