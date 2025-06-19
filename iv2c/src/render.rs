use std::time::Duration;

use image::DynamicImage;

use crate::{error::Error, frames::FrameIterator, pipeline::ImagePipeline};

pub struct RenderFrame {
    pub text: String,
    pub colors: Vec<u8>,
}

pub struct CallbackState<'a> {
    pub frame: Option<RenderFrame>,
    pub should_render: bool,
    pub pipeline: &'a mut ImagePipeline,
}

#[cfg(feature = "render")]
impl RenderFrame {
    pub fn render_to_image(&self, font_px: f32, background_color: &[u8; 4]) -> image::RgbaImage {
        use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

        let height = self.text.lines().count();

        let img_width = (self
            .text
            .lines()
            .next()
            .map(|line| line.chars().count())
            .unwrap_or(0) as f32
            * font_px)
            .ceil() as u32;

        let img_height = (height as f32 * font_px).ceil() as u32;

        let font_data = include_bytes!("JetBrainsMono-Regular.ttf");
        let font = ab_glyph::FontRef::try_from_slice(font_data.as_slice()).unwrap();

        let mut color_idx = 0;
        let lines_data: Vec<(String, &[u8])> = self
            .text
            .lines()
            .map(|line| {
                let line_len = line.chars().count();
                let color_slice = &self.colors[color_idx..color_idx + 3 * line_len];
                color_idx += 3 * line_len + 6;
                (line.to_string(), color_slice)
            })
            .collect();

        let line_images: Vec<image::RgbaImage> = lines_data
            .par_iter()
            .map(|(line, colors)| {
                use image::{Pixel, Rgba};

                let line_width = (line.chars().count() as f32 * font_px).ceil() as u32;
                let line_height = font_px.ceil() as u32;

                let mut img = image::RgbaImage::from_pixel(
                    line_width,
                    line_height,
                    *Rgba::<u8>::from_slice(background_color),
                );

                let mut local_color_idx = 0;
                for (x, ch) in line.chars().enumerate() {
                    if local_color_idx + 2 >= colors.len() {
                        break;
                    }
                    let color = image::Rgba([
                        colors[local_color_idx],
                        colors[local_color_idx + 1],
                        colors[local_color_idx + 2],
                        255,
                    ]);
                    imageproc::drawing::draw_text_mut(
                        &mut img,
                        color,
                        (x as f32 * font_px) as i32,
                        0,
                        font_px,
                        &font,
                        &ch.to_string(),
                    );
                    local_color_idx += 3;
                }
                img
            })
            .collect();

        let mut image = image::RgbaImage::new(img_width, img_height);
        for (i, line_img) in line_images.into_iter().enumerate() {
            let y_offset = (i as f32 * font_px) as u32;
            for (x, y, pixel) in line_img.enumerate_pixels() {
                image.put_pixel(x, y + y_offset, *pixel);
            }
        }

        image
    }
}

impl From<(String, Vec<u8>)> for RenderFrame {
    fn from(val: (String, Vec<u8>)) -> Self {
        RenderFrame {
            text: val.0,
            colors: val.1,
        }
    }
}

pub struct Renderer {
    /// The image pipeline responsible for processing images.
    pub pipeline: ImagePipeline,
    /// The FrameIterator that handles iterating through frames.
    media: FrameIterator,
    /// The last frame that was processed by the Renderer.
    last_frame: Option<DynamicImage>,
    /// Render options
    render_options: RenderOptions,
}

pub struct RenderOptions {
    /// The target frames per second (frame rate) for the Renderer.
    pub fps: f64,
    /// The width modifier (use 2 for emojis).
    pub w_mod: u32,
    /// loop back to the first frame after iterating through frames.
    pub loop_playback: bool,
}

impl Renderer {
    pub fn new(
        pipeline: ImagePipeline,
        media: FrameIterator,
        render_options: RenderOptions,
    ) -> Self {
        Self {
            pipeline,
            media,
            last_frame: None,
            render_options,
        }
    }

    pub fn run(
        &mut self,
        allow_frame_skip: bool,
        callback: impl Fn(CallbackState) -> bool,
    ) -> Result<(), Error> {
        let mut time_count = std::time::Instant::now();
        let mut should_continue = true;

        while should_continue {
            let (should_process_frame, frames_to_skip) = self.should_process_frame(&mut time_count);
            let frame = if should_process_frame {
                if allow_frame_skip && frames_to_skip > 0 {
                    self.media.skip_frames(frames_to_skip);
                }
                let f = self.get_current_frame();

                if self.render_options.loop_playback && f.is_none() {
                    // make sure the first frame on replay is shown immediately
                    time_count -= self.target_frame_duration();
                    // replay
                    self.replay_pipeline();
                }

                self.render_current_frame(f.as_ref())
            } else {
                None
            };

            should_continue = callback(CallbackState {
                frame,
                should_render: should_process_frame,
                pipeline: &mut self.pipeline,
            });
        }

        Ok(())
    }

    fn should_process_frame(&self, time_count: &mut std::time::Instant) -> (bool, usize) {
        let (time_to_send_next_frame, frames_to_skip) = self.time_to_send_next_frame(time_count);

        if time_to_send_next_frame {
            (true, frames_to_skip)
        } else {
            (false, 0)
        }
    }

    fn replay_pipeline(&mut self) {
        self.media.reset();
    }

    fn time_to_send_next_frame(&self, time_count: &mut std::time::Instant) -> (bool, usize) {
        let elapsed_time = time_count.elapsed();
        let target_frame_duration = self.target_frame_duration();

        if elapsed_time >= target_frame_duration {
            let frames_to_skip =
                (elapsed_time.as_nanos() / target_frame_duration.as_nanos()) as usize - 1;
            *time_count += target_frame_duration * (frames_to_skip as u32 + 1);
            (true, frames_to_skip)
        } else {
            (false, 0)
        }
    }

    fn target_frame_duration(&self) -> Duration {
        // if negative, will have no frame duration (instant)
        Duration::from_nanos((1_000_000_000_f64 / self.render_options.fps.max(0_f64)) as u64)
    }

    pub fn render_frame(&mut self, frame: &DynamicImage) -> Result<RenderFrame, Error> {
        let procimage = self.pipeline.resize(frame)?;
        let width = procimage.width();
        let grayimage = procimage.clone().into_luma8();
        let rgb_info = procimage.into_rgb8().to_vec();

        if self.pipeline.new_lines {
            let mut rgb_info_newline = Vec::with_capacity(rgb_info.len() + 6 * width as usize);

            for (i, pixel) in rgb_info.chunks(3).enumerate() {
                rgb_info_newline.extend_from_slice(pixel);
                if (i + 1) % width as usize == 0 {
                    rgb_info_newline.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
                }
            }
            return Ok((self.pipeline.to_ascii(&grayimage), rgb_info_newline).into());
        }
        Ok((self.pipeline.to_ascii(&grayimage), rgb_info).into())
    }

    fn render_current_frame(&mut self, frame: Option<&DynamicImage>) -> Option<RenderFrame> {
        match frame {
            Some(frame) => {
                self.last_frame = Some(frame.clone());
                if let Ok(render_frame) = self.render_frame(frame) {
                    return Some(render_frame);
                }
                None
            }
            None => {
                if self.last_frame.is_some()
                    && let Ok(render_frame) = self.render_frame(
                        &self
                            .last_frame
                            .clone()
                            .expect("Last frame should be available"),
                    )
                {
                    return Some(render_frame);
                }
                None
            }
        }
    }

    fn get_current_frame(&mut self) -> Option<DynamicImage> {
        self.media.next()
    }
}
