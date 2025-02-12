use core::convert::TryFrom;

use crate::error::{Error, Result};
use crate::utils::unlikely;

/// Image color space.
///
/// Note: the color space is purely informative. Although it is saved to the
/// file header, it does not affect encoding/decoding in any way.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
#[repr(u8)]
pub enum ColorSpace {
    /// sRGB with linear alpha
    Srgb = 0,
    /// All channels are linear
    Linear = 1,
}

impl ColorSpace {
    /// Returns true if the color space is sRGB with linear alpha.
    pub const fn is_srgb(self) -> bool {
        matches!(self, Self::Srgb)
    }

    /// Returns true is all channels are linear.
    pub const fn is_linear(self) -> bool {
        matches!(self, Self::Linear)
    }

    /// Converts to an integer (0 if sRGB, 1 if all linear).
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

impl Default for ColorSpace {
    fn default() -> Self {
        Self::Srgb
    }
}

impl From<ColorSpace> for u8 {
    #[inline]
    fn from(colorspace: ColorSpace) -> Self {
        colorspace as Self
    }
}

impl TryFrom<u8> for ColorSpace {
    type Error = Error;

    #[inline]
    fn try_from(colorspace: u8) -> Result<Self> {
        if unlikely(colorspace | 1 != 1) {
            Err(Error::InvalidColorSpace { colorspace })
        } else {
            Ok(if colorspace == 0 { Self::Srgb } else { Self::Linear })
        }
    }
}

/// Number of 8-bit channels in a pixel.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
#[repr(u8)]
pub enum Channels {
    /// Three 8-bit channels (RGB)
    Rgb = 3,
    /// Four 8-bit channels (RGBA)
    Rgba = 4,
}

impl Channels {
    /// Returns true if there are 3 channels (RGB).
    pub const fn is_rgb(self) -> bool {
        matches!(self, Self::Rgb)
    }

    /// Returns true if there are 4 channels (RGBA).
    pub const fn is_rgba(self) -> bool {
        matches!(self, Self::Rgba)
    }

    /// Converts to an integer (3 if RGB, 4 if RGBA).
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

impl Default for Channels {
    fn default() -> Self {
        Self::Rgb
    }
}

impl From<Channels> for u8 {
    #[inline]
    fn from(channels: Channels) -> Self {
        channels as Self
    }
}

impl TryFrom<u8> for Channels {
    type Error = Error;

    #[inline]
    fn try_from(channels: u8) -> Result<Self> {
        if unlikely(channels != 3 && channels != 4) {
            Err(Error::InvalidChannels { channels })
        } else {
            Ok(if channels == 3 { Self::Rgb } else { Self::Rgba })
        }
    }
}

/// Pixel format for the source image.
///
/// The layout does not depend on the endianness of the system.
/// The components are stored as bytes in the given order.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SourceChannels {
    /// Pixel is R, G and B channels
    Rgb,
    /// Pixel is B, G and R channels
    Bgr,
    /// Pixel is RGB with an alpha channel
    Rgba,
    /// Pixel is an alpha channel and RGB
    Argb,
    /// Pixel is RGB with an extra byte
    Rgbx,
    /// Pixel is an extra byte and RGB
    Xrgb,
    /// Pixel is BGR with an alpha channel
    Bgra,
    /// Pixel is an alpha channel and BGR
    Abgr,
    /// Pixel is BGR with an extra byte
    Bgrx,
    /// Pixel is an extra byte and BGR
    Xbgr,
}

impl From<Channels> for SourceChannels {
    fn from(value: Channels) -> Self {
        match value {
            Channels::Rgb => Self::Rgb,
            Channels::Rgba => Self::Rgba,
        }
    }
}

impl SourceChannels {
    pub(crate) const fn image_channels(self) -> Channels {
        match self {
            Self::Rgb | Self::Bgr | Self::Rgbx | Self::Xrgb | Self::Bgrx | Self::Xbgr => {
                Channels::Rgb
            }
            Self::Rgba | Self::Argb | Self::Bgra | Self::Abgr => Channels::Rgba,
        }
    }

    pub(crate) const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Rgb | Self::Bgr => 3,
            Self::Rgba
            | Self::Argb
            | Self::Rgbx
            | Self::Xrgb
            | Self::Bgra
            | Self::Abgr
            | Self::Bgrx
            | Self::Xbgr => 4,
        }
    }
}
