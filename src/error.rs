use core::convert::Infallible;
use core::fmt::{self, Display};

use crate::consts::{QOI_MAGIC, QOI_PIXELS_MAX};

#[derive(Debug)]
pub enum Error {
    InvalidChannels {
        channels: u8,
    },
    EmptyImage {
        width: u32,
        height: u32,
    },
    ImageTooLarge {
        width: u32,
        height: u32,
    },
    InvalidImageLength {
        size: usize,
        width: u32,
        height: u32,
    },
    InputBufferTooSmall {
        size: usize,
        required: usize,
    },
    OutputBufferTooSmall {
        size: usize,
        required: usize,
    },
    InvalidMagic {
        magic: u32,
    },
    UnexpectedBufferEnd,
    InvalidColorSpace {
        colorspace: u8,
    },
    InvalidPadding,
    #[cfg(feature = "std")]
    IoError(std::io::Error),
}

pub type Result<T> = core::result::Result<T, Error>;

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
            Self::InvalidImageLength { size, width, height } => {
                write!(f, "invalid image length: {} for {}x{}", size, width, height)
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
            Self::InvalidColorSpace { colorspace } => {
                write!(f, "invalid color space: {} (expected 0 or 1)", colorspace)
            }
            Self::InvalidPadding => {
                write!(f, "invalid padding (stream end marker)")
            }
            #[cfg(feature = "std")]
            Self::IoError(ref err) => {
                write!(f, "i/o error: {}", err)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}
