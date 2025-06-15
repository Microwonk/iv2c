use crate::{error::*, maps::CharMap};
use fast_image_resize as fir;
use image::{DynamicImage, GrayImage};

pub enum Resolution {
    /// Use fixed resolution (width, height)
    Fixed(u32, u32),
    /// Use a divisor to scale resolution while preserving aspect ratio
    Divisor(u32),
}

impl Resolution {
    pub fn calc(&self, img: &DynamicImage) -> (u32, u32) {
        match self {
            Resolution::Fixed(w, h) => (*w, *h),
            Resolution::Divisor(d) => (img.width() / d, img.height() / d),
        }
    }
}

pub struct ImagePipeline {
    /// The target resolution (width and height) for the pipeline.
    pub resolution: Resolution,
    /// The character lookup table used for the conversion.
    pub char_map: Vec<char>,
    /// Whether to add newlines to the output at the end of each line
    pub new_lines: bool,
}

impl ImagePipeline {
    pub fn new(resolution: Resolution, char_map: CharMap, new_lines: bool) -> Self {
        Self {
            resolution,
            char_map: char_map.chars(),
            new_lines,
        }
    }

    pub fn set_resolution(&mut self, resolution: Resolution) -> &mut Self {
        self.resolution = resolution;
        self
    }

    pub fn resize(&self, img: &DynamicImage) -> Result<DynamicImage, Error> {
        let width = img.width();
        let height = img.height();
        let src_image = fir::images::Image::from_vec_u8(
            width,
            height,
            img.to_owned().into_rgb8().to_vec(),
            fir::PixelType::U8x3,
        )
        .map_err(|err| Error::Pipeline(format!("{ERROR_RESIZE}:{err:?}")))?;

        let (dst_w, dst_h) = self.resolution.calc(img);
        let mut dst_image = fir::images::Image::new(dst_w, dst_h, fir::PixelType::U8x3);

        let mut resizer = fir::Resizer::new();
        resizer
            .resize(
                &src_image,
                &mut dst_image,
                &fir::ResizeOptions::new().resize_alg(fir::ResizeAlg::Nearest),
            )
            .map_err(|err| Error::Pipeline(format!("{ERROR_RESIZE}:{err:?}")))?;

        let dst_image = dst_image.into_vec();
        let img_buff = image::ImageBuffer::<image::Rgb<u8>, _>::from_vec(dst_w, dst_h, dst_image)
            .ok_or(Error::Pipeline(ERROR_DATA.to_string()))?;

        Ok(DynamicImage::ImageRgb8(img_buff))
    }

    pub fn to_ascii(&self, input: &GrayImage) -> String {
        let (width, height) = (input.width(), input.height());
        let capacity = (width + 1) * height + 1;
        let mut output = String::with_capacity(capacity as usize);

        for y in 0..height {
            output.extend((0..width).map(|x| {
                let lum = input.get_pixel(x, y)[0] as u32;
                let lookup_idx = self.char_map.len() * lum as usize / (u8::MAX as usize + 1);
                self.char_map[lookup_idx]
            }));

            if self.new_lines && y < height - 1 {
                output.push('\r');
                output.push('\n');
            }
        }

        output
    }
}
