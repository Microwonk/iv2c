//! This module defines the custom error type `Error` used throughout the application,
//! as well as various error message constants.
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Application error: {0}")]
    Application(String),
    #[error("Image pipeline error: {0}")]
    Pipeline(String),
}

impl From<Error> for io::Error {
    fn from(error: Error) -> Self {
        io::Error::other(error.to_string())
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Application(format!("{error}"))
    }
}

impl From<opencv::Error> for Error {
    fn from(error: opencv::Error) -> Self {
        Error::Application(format!("{error}"))
    }
}

/// Error message for issues related to decoding an image.
pub const ERROR_DECODING_IMAGE: &str = "Error decoding image";
/// Error message for issues related to opening a video.
pub const ERROR_OPENING_VIDEO: &str = "Error opening video";
/// Error message for issues related to opening a resource.
pub const ERROR_OPENING_RESOURCE: &str = "Error opening resource";
/// Error message for issues related to reading a GIF header.
pub const ERROR_READING_GIF_HEADER: &str = "Cannot read GIF header";
/// Error message for issues related to data processing.
pub const ERROR_DATA: &str = "Data error";
/// Error message for issues related to resizing an image.
pub const ERROR_RESIZE: &str = "Image resizing error";
