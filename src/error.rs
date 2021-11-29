use std::error::Error as StdError;
use std::fmt::{self, Display};
use std::result::Result as StdResult;

use crate::consts::{QOI_HEADER_SIZE, QOI_MAGIC, QOI_PADDING};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidChannels { channels: u8 },
    EmptyImage { width: u32, height: u32 },
    BadEncodingDataSize { size: usize, expected: usize },
    BadDecodingDataSize { size: usize },
    InvalidMagic { magic: u32 },
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
            Self::BadEncodingDataSize { size, expected } => {
                write!(f, "bad data size when encoding: {} (expected: {})", size, expected)
            }
            Self::BadDecodingDataSize { size } => {
                let min_size = QOI_HEADER_SIZE + QOI_PADDING;
                write!(f, "bad data size when decoding: {} (minimum required: {})", size, min_size)
            }
            Self::InvalidMagic { magic } => {
                write!(f, "invalid magic: expected {:?}, got {:?}", QOI_MAGIC, magic)
            }
        }
    }
}

impl StdError for Error {}
