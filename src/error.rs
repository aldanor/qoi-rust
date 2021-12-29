use std::error::Error as StdError;
use std::fmt::{self, Display};
use std::result::Result as StdResult;

use crate::consts::{QOI_MAGIC, QOI_PIXELS_MAX};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidChannels { channels: u8 },
    EmptyImage { width: u32, height: u32 },
    ImageTooLarge { width: u32, height: u32 },
    BadEncodingDataSize { size: usize, expected: usize },
    InputBufferTooSmall { size: usize, required: usize },
    OutputBufferTooSmall { size: usize, required: usize },
    InvalidMagic { magic: u32 },
    UnexpectedBufferEnd,
    // TODO: invalid colorspace
}

pub type Result<T> = StdResult<T, Error>;

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::InvalidChannels { channels } => {
                write!(f, "invalid number of channels: {}", channels)
            }
            Self::EmptyImage { width, height } => {
                write!(f, "image contains no pixels: {}x{}", width, height)
            }
            Self::ImageTooLarge { width, height } => {
                let mp = QOI_PIXELS_MAX / 1_000_000;
                write!(f, "image is too large: {}x{} (max={}Mp)", width, height, mp)
            }
            Self::BadEncodingDataSize { size, expected } => {
                write!(f, "bad data size when encoding: {} (expected: {})", size, expected)
            }
            Self::InputBufferTooSmall { size, required } => {
                write!(f, "input buffer size too small: {} (minimum required: {})", size, required)
            }
            Self::OutputBufferTooSmall { size, required } => {
                write!(f, "output buffer size too small: {} (minimum required: {})", size, required)
            }
            Self::InvalidMagic { magic } => {
                write!(f, "invalid magic: expected {:?}, got {:?}", QOI_MAGIC, magic)
            }
            Self::UnexpectedBufferEnd => {
                write!(f, "unexpected input buffer end while decoding")
            }
        }
    }
}

impl StdError for Error {}
