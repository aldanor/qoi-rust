use std::convert::TryInto;

use bytemuck::cast_slice;

use crate::consts::{QOI_HEADER_SIZE, QOI_MAGIC, QOI_PIXELS_MAX};
use crate::encoded_size_limit;
use crate::error::{Error, Result};
use crate::types::{Channels, ColorSpace};
use crate::utils::unlikely;

/// QOI image header.
///
/// ### Notes
/// A valid image header must satisfy the following conditions:
/// * Both width and height must be non-zero.
/// * Maximum number of pixels is 400Mp (=4e8 pixels).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Header {
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Number of 8-bit channels per pixel
    pub channels: Channels,
    /// Color space (informative field, doesn't affect encoding)
    pub colorspace: ColorSpace,
}

impl Default for Header {
    #[inline]
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
            channels: Channels::default(),
            colorspace: ColorSpace::default(),
        }
    }
}

impl Header {
    /// Creates a new header and validates image dimensions.
    #[inline]
    pub const fn try_new(
        width: u32, height: u32, channels: Channels, colorspace: ColorSpace,
    ) -> Result<Self> {
        if unlikely(height == 0 || width == 0) {
            return Err(Error::EmptyImage { width, height });
        } else if unlikely((width as usize).saturating_mul(height as usize) > QOI_PIXELS_MAX) {
            return Err(Error::ImageTooLarge { width, height });
        }
        Ok(Self { width, height, channels, colorspace })
    }

    /// Creates a new header with modified channels.
    #[inline]
    pub const fn with_channels(mut self, channels: Channels) -> Self {
        self.channels = channels;
        self
    }

    /// Creates a new header with modified color space.
    #[inline]
    pub const fn with_colorspace(mut self, colorspace: ColorSpace) -> Self {
        self.colorspace = colorspace;
        self
    }

    /// Serializes the header into a bytes array.
    #[inline]
    pub(crate) fn encode(&self) -> [u8; QOI_HEADER_SIZE] {
        let mut out = [0; QOI_HEADER_SIZE];
        out[..4].copy_from_slice(&QOI_MAGIC.to_be_bytes());
        out[4..8].copy_from_slice(&self.width.to_be_bytes());
        out[8..12].copy_from_slice(&self.height.to_be_bytes());
        out[12] = self.channels.into();
        out[13] = self.colorspace.into();
        out
    }

    /// Deserializes the header from a byte array.
    #[inline]
    pub(crate) fn decode(data: impl AsRef<[u8]>) -> Result<Self> {
        let data = data.as_ref();
        if unlikely(data.len() < QOI_HEADER_SIZE) {
            return Err(Error::InputBufferTooSmall { size: data.len(), required: QOI_HEADER_SIZE });
        }
        let v = cast_slice::<_, [u8; 4]>(&data[..12]);
        let magic = u32::from_be_bytes(v[0]);
        let width = u32::from_be_bytes(v[1]);
        let height = u32::from_be_bytes(v[2]);
        let channels = data[12].try_into()?;
        let colorspace = data[13].try_into()?;
        if unlikely(magic != QOI_MAGIC) {
            return Err(Error::InvalidMagic { magic });
        }
        Self::try_new(width, height, channels, colorspace)
    }

    /// Returns a number of pixels in the image.
    #[inline]
    pub const fn n_pixels(&self) -> usize {
        (self.width as usize).saturating_mul(self.height as usize)
    }

    /// Returns the total number of bytes in the image.
    #[inline]
    pub const fn n_bytes(&self) -> usize {
        self.n_pixels() * self.channels.as_u8() as usize
    }

    /// Returns the maximum number of bytes the image can possibly occupy when QOI-encoded.
    ///
    /// This comes useful when pre-allocating a buffer to encode the image into.
    #[inline]
    pub fn encoded_size_limit(&self) -> usize {
        encoded_size_limit(self.width, self.height, self.channels)
    }
}
