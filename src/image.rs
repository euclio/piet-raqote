use piet::{
    Image,
    kurbo::{Rect, Size},
};
use raqote::DrawTarget;

/// Analogue of [`raqote::Image`] that owns its data.
#[derive(Debug, Clone)]
struct OwnedImage {
    data: Vec<u32>,
    width: i32,
    height: i32,
}

#[derive(Debug, Clone)]
pub struct RaqoteImage(OwnedImage);

impl RaqoteImage {
    pub fn new(width: i32, height: i32, data: Vec<u32>) -> Self {
        RaqoteImage(OwnedImage {
            width,
            height,
            data,
        })
    }

    /// Create a new image from a subregion of another image (or the draw target).
    pub(crate) fn from_region(src: impl AsImage, src_rect: impl Into<Rect>) -> Self {
        let src_image = src.as_image();
        let src_rect = src_rect.into();

        let src_x = src_rect.origin().x as usize;
        let src_y = src_rect.origin().y as usize;
        let src_width = src_rect.width() as usize;
        let src_height = src_rect.height() as usize;

        let mut output = vec![0u32; src_rect.area() as usize];

        for (i, row) in src_image
            .data
            .chunks_exact(src_image.width as usize)
            .skip(src_y)
            .take(src_height)
            .enumerate()
        {
            let src_row_slice = &row[src_x..src_x + src_width];
            let dst_row_slice = &mut output[i * src_width..(i + 1) * src_width];

            dst_row_slice.copy_from_slice(src_row_slice);
        }

        RaqoteImage::new(src_width as i32, src_height as i32, output)
    }
}

impl Image for RaqoteImage {
    fn size(&self) -> piet::kurbo::Size {
        Size::new(self.0.width.into(), self.0.height.into())
    }
}

/// Image-like types.
pub(crate) trait AsImage {
    fn as_image(&self) -> raqote::Image;
}

impl AsImage for RaqoteImage {
    fn as_image(&self) -> raqote::Image {
        raqote::Image {
            width: self.0.width,
            height: self.0.height,
            data: &self.0.data,
        }
    }
}

impl<B> AsImage for DrawTarget<B>
where
    B: AsRef<[u32]> + AsMut<[u32]>,
{
    fn as_image(&self) -> raqote::Image {
        raqote::Image {
            width: self.width(),
            height: self.height(),
            data: self.get_data(),
        }
    }
}

impl<T: AsImage> AsImage for &T {
    fn as_image(&self) -> raqote::Image {
        (**self).as_image()
    }
}
