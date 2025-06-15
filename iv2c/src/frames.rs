use crate::{
    error::*,
    util::{extract_fps, mat_to_dynamic_image},
};
use gif;
use image::{DynamicImage, ImageReader};
use libwebp_sys as webp;
use opencv::{prelude::*, videoio::VideoCapture};
use std::{fs::File, io::Read, path::Path};

#[derive(Debug)]
pub enum FrameIterator {
    Image(Option<DynamicImage>),
    Video(VideoCapture),
    AnimatedImage {
        frames: Vec<DynamicImage>,
        current_frame: usize,
    },
}

#[derive(Debug)]
pub struct MediaData {
    pub frame_iter: FrameIterator,
    pub fps: Option<f64>,
}

impl Iterator for FrameIterator {
    type Item = DynamicImage;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            FrameIterator::Image(img) => img.take(),
            FrameIterator::Video(video) => capture_video_frame(video),
            FrameIterator::AnimatedImage {
                frames,
                current_frame,
            } => {
                if *current_frame == frames.len() {
                    None
                } else {
                    let frame = frames.get(*current_frame).cloned();
                    *current_frame += 1;
                    frame
                }
            }
        }
    }
}

impl FrameIterator {
    pub fn skip_frames(&mut self, n: usize) {
        match self {
            FrameIterator::Image(_) => {
                // For a single image, skipping is a no-op, since there's only one frame
            }
            FrameIterator::Video(video) => {
                for _ in 0..n {
                    let mut frame = Mat::default();
                    if !video.read(&mut frame).unwrap_or(false) || frame.empty() {
                        break;
                    }
                }
            }
            FrameIterator::AnimatedImage {
                current_frame,
                frames,
            } => {
                *current_frame = (*current_frame + n) % frames.len();
            }
        }
    }

    pub fn reset(&mut self) {
        match self {
            FrameIterator::Image(_) => {
                // For a single image, reset is a no-op, since there's only one frame
            }
            FrameIterator::Video(video) => {
                let _ = video.set(opencv::videoio::CAP_PROP_POS_AVI_RATIO, 0.0);
            }
            FrameIterator::AnimatedImage { current_frame, .. } => {
                *current_frame = 0;
            }
        }
    }
}

pub fn open_media_from_path(path: &Path) -> Result<MediaData, Error> {
    let fps = extract_fps(path);

    let ext = path.extension().and_then(std::ffi::OsStr::to_str);
    match ext {
        // Image extensions
        Some("png") | Some("bmp") | Some("ico") | Some("tif") | Some("tiff") | Some("jpg")
        | Some("jpeg") => Ok(MediaData {
            frame_iter: open_image(path)?,
            fps: None,
        }),
        // Video extensions
        Some("mp4") | Some("avi") | Some("webm") | Some("mkv") | Some("mov") | Some("flv")
        | Some("ogg") => Ok(MediaData {
            frame_iter: open_video(path)?,
            fps,
        }),
        // Gif
        Some("gif") => {
            let (frame_iter, fps) = open_gif(path)?;
            Ok(MediaData {
                frame_iter,
                fps: Some(fps),
            })
        }
        // Webp
        Some("webp") => {
            let (frame_iter, fps) = open_webp(path)?;
            Ok(MediaData {
                frame_iter,
                fps: Some(fps),
            })
        }
        // Unknown extension, try open as video
        _ => Ok(MediaData {
            frame_iter: open_video(path)?,
            fps,
        }),
    }
}

fn capture_video_frame(video: &mut VideoCapture) -> Option<DynamicImage> {
    let mut frame = Mat::default();
    if video.read(&mut frame).unwrap_or(false) && !frame.empty() {
        mat_to_dynamic_image(&frame)
    } else {
        None
    }
}

fn open_image(path: &Path) -> Result<FrameIterator, Error> {
    let img = ImageReader::open(path)?
        .decode()
        .map_err(|e| Error::Application(format!("{ERROR_DECODING_IMAGE}: {e:?}")))?;
    Ok(FrameIterator::Image(Some(img)))
}

fn open_video(path: &Path) -> Result<FrameIterator, Error> {
    let video = VideoCapture::from_file(
        path.to_str().expect(ERROR_OPENING_VIDEO),
        opencv::videoio::CAP_ANY,
    )?;

    if video.is_opened()? {
        Ok(FrameIterator::Video(video))
    } else {
        Err(Error::Application(ERROR_OPENING_VIDEO.to_string()))
    }
}

fn open_gif(path: &Path) -> Result<(FrameIterator, f64), Error> {
    let file = File::open(path)
        .map_err(|e| Error::Application(format!("{ERROR_OPENING_RESOURCE}: {e:?}")))?;
    let mut options = gif::DecodeOptions::new();
    // https://lib.rs/crates/gif-dispose
    // for gif_dispose frame composing for rgba output, we need to set this as indexed.
    options.set_color_output(gif::ColorOutput::Indexed);
    let mut decoder = options
        .read_info(file)
        .map_err(|e| Error::Application(format!("{ERROR_READING_GIF_HEADER}: {e:?}")))?;

    // delay is in units of 10ms, so we'll divide by 100.0, not 1000.0
    let mut delay: u64 = 0;
    let mut frames = Vec::new();
    // The gif crate only exposes raw frame data that is not sufficient to render animated GIFs properly.
    // GIF requires special composing of frames which is non-trivial.
    let mut screen = gif_dispose::Screen::new_decoder(&decoder);
    while let Ok(Some(frame)) = decoder.read_next_frame() {
        delay += frame.delay as u64;
        screen
            .blit_frame(frame)
            .map_err(|e| Error::Application(format!("{ERROR_DECODING_IMAGE}: {e:?}")))?;
        let (buf, width, height) = screen.pixels_rgba().to_contiguous_buf();
        frames.push(DynamicImage::ImageRgba8(image::RgbaImage::from_fn(
            width as u32,
            height as u32,
            |x, y| {
                let rgba = buf.as_ref()[y as usize * width + x as usize];
                image::Rgba([rgba.r, rgba.g, rgba.b, rgba.a])
            },
        )));
    }

    // fps is only an average across all frames, there is no per frame delay modelling
    let fps = frames.len() as f64 / (delay.max(1) as f64 / 100.0);
    Ok((
        FrameIterator::AnimatedImage {
            frames,
            current_frame: 0,
        },
        fps,
    ))
}

fn open_webp(path: &Path) -> Result<(FrameIterator, f64), Error> {
    let mut file = File::open(path)
        .map_err(|e| Error::Application(format!("{ERROR_OPENING_RESOURCE}: {e:?}")))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    let mut frames = Vec::new();
    let mut first_timestamp: i32 = i32::MAX;
    let mut last_timestamp: i32 = i32::MIN;
    // this code is based on the code example here:
    // https://developers.google.com/speed/webp/docs/container-api#webpanimdecoder_api
    unsafe {
        let mut options = webp::WebPAnimDecoderOptions {
            color_mode: webp::WEBP_CSP_MODE::MODE_RGBA,
            use_threads: 0,
            padding: [0, 0, 0, 0, 0, 0, 0],
        };
        webp::WebPAnimDecoderOptionsInit(&mut options);
        let dec = webp::WebPAnimDecoderNew(
            &webp::WebPData {
                bytes: buf.as_ptr(),
                size: buf.len(),
            },
            &options,
        );
        let mut info = webp::WebPAnimInfo::default();
        webp::WebPAnimDecoderGetInfo(dec, &mut info);
        let frame_sz = (info.canvas_width * info.canvas_height * 4) as usize;
        for _ in 0..info.loop_count {
            while webp::WebPAnimDecoderHasMoreFrames(dec) != 0 {
                let mut buf: *mut u8 = std::ptr::null_mut();
                let mut timestamp: i32 = 0;
                webp::WebPAnimDecoderGetNext(dec, &mut buf, &mut timestamp);
                first_timestamp = first_timestamp.min(timestamp);
                last_timestamp = last_timestamp.max(timestamp);
                if let Some(image) = image::RgbaImage::from_raw(
                    info.canvas_width,
                    info.canvas_height,
                    std::slice::from_raw_parts(buf, frame_sz).to_vec(),
                ) {
                    frames.push(DynamicImage::ImageRgba8(image));
                } else {
                    // eprintln!("Failed to decode frame");
                }
            }
            webp::WebPAnimDecoderReset(dec);
        }
        webp::WebPAnimDecoderDelete(dec);
    }

    // fps is only an average across all frames, there is no per frame delay modelling
    let fps = frames.len() as f64
        / ((last_timestamp.saturating_sub(first_timestamp).max(1)) as f64 / 1000.0);
    Ok((
        FrameIterator::AnimatedImage {
            frames,
            current_frame: 0,
        },
        fps,
    ))
}
