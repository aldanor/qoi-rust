use core::convert::Infallible;
use core::fmt::{self, Display};

use crate::consts::QOI_MAGIC;

/// Errors that can occur during encoding or decoding.
#[derive(Debug)]
pub enum Error {
    /// Leading 4 magic bytes don't match when decoding
    InvalidMagic { magic: u32 },
    /// Invalid number of channels: expected 3 or 4
    InvalidChannels { channels: u8 },
    /// Invalid color space: expected 0 or 1
    InvalidColorSpace { colorspace: u8 },
    /// Invalid image dimensions: can't be empty or larger than 400Mp
    InvalidImageDimensions { width: u32, height: u32 },
    /// Image dimensions are inconsistent with image buffer length
    InvalidImageLength { size: usize, width: u32, height: u32 },
    /// Image stride is inconsistent with image dimension and buffer length
    InvalidImageStride { size: usize, width: u32, height: u32, stride: usize },
    /// Output buffer is too small to fit encoded/decoded image
    OutputBufferTooSmall { size: usize, required: usize },
    /// Input buffer ended unexpectedly before decoding was finished
    UnexpectedBufferEnd,
    /// Invalid stream end marker encountered when decoding
    InvalidPadding,
    #[cfg(feature = "std")]
    /// Generic I/O error from the wrapped reader/writer
    IoError(std::io::Error),
}

/// Alias for [`Result`](std::result::Result) with the error type of [`Error`].
pub type Result<T> = core::result::Result<T, Error>;

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::InvalidMagic { magic } => {
                write!(f, "invalid magic: expected {:?}, got {:?}", QOI_MAGIC, magic.to_be_bytes())
            }
            Self::InvalidChannels { channels } => {
                write!(f, "invalid number of channels: {channels}")
            }
            Self::InvalidColorSpace { colorspace } => {
                write!(f, "invalid color space: {colorspace} (expected 0 or 1)")
            }
            Self::InvalidImageDimensions { width, height } => {
                write!(f, "invalid image dimensions: {width}x{height}")
            }
            Self::InvalidImageLength { size, width, height } => {
                write!(f, "invalid image length: {size} bytes for {width}x{height}")
            }
            Self::InvalidImageStride { size, width, height, stride } => {
                write!(f, "invalid image stride: {stride} for {size} bytes of {width}x{height}")
            }
            Self::OutputBufferTooSmall { size, required } => {
                write!(f, "output buffer size too small: {size} (required: {required})")
            }
            Self::UnexpectedBufferEnd => {
                write!(f, "unexpected input buffer end while decoding")
            }
            Self::InvalidPadding => {
                write!(f, "invalid padding (stream end marker mismatch)")
            }
            #[cfg(feature = "std")]
            Self::IoError(ref err) => {
                write!(f, "i/o error: {err}")
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
