use std::{
    path::Path,
    process::{Command, Stdio},
    str::FromStr,
};

use image::{DynamicImage, ImageBuffer};
use num::{Rational64, ToPrimitive};
use opencv::{core::*, imgproc};
use serde_json::Value;

pub fn extract_fps(video_path: &Path) -> Option<f64> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("v:0")
        .arg("-show_entries")
        .arg("stream=r_frame_rate")
        .arg("-of")
        .arg("json")
        .arg(video_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .expect("Failed to extract fps from video. Is ffprobe installed?");

    let output_str = String::from_utf8(output.stdout).unwrap_or("".to_string());
    let json_value: Value = serde_json::from_str(&output_str).unwrap_or(Value::Null);
    if json_value != Value::Null {
        let r_frame_rate = json_value["streams"][0]["r_frame_rate"]
            .as_str()
            .ok_or("".to_string())
            .unwrap_or("");

        let frame_rate_f = Rational64::from_str(r_frame_rate);
        if let Ok(frame_rate) = frame_rate_f {
            return Some(frame_rate.to_f64().expect("Failed to parse FPS value"));
        }
    }

    None
}

pub fn mat_to_dynamic_image(mat: &Mat) -> Option<DynamicImage> {
    let mut rgb_mat = Mat::default();
    if imgproc::cvt_color(
        &mat,
        &mut rgb_mat,
        imgproc::COLOR_BGR2RGB,
        0,
        AlgorithmHint::ALGO_HINT_DEFAULT,
    )
    .is_ok()
        && rgb_mat.elem_size().is_ok()
        && let Ok(size) = rgb_mat.size()
    {
        let reshaped_mat = rgb_mat.reshape(1, size.width * size.height).ok()?;
        let data_vec: Vec<u8> = reshaped_mat
            .data_typed::<u8>()
            .expect("Unexpected invalid data")
            .to_vec();

        if let Some(img_buf) = ImageBuffer::<image::Rgb<u8>, _>::from_raw(
            size.width as u32,
            size.height as u32,
            data_vec,
        ) {
            return Some(DynamicImage::ImageRgb8(img_buf));
        }
    }
    None
}
