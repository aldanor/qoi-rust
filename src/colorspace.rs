use std::convert::TryFrom;

use crate::error::{Error, Result};
use crate::utils::unlikely;

/// Image color space.
///
/// Note: the color space is purely informative. Although it is saved to the
/// file header, it does not affect encoding/decoding in any way.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
#[repr(u8)]
pub enum ColorSpace {
    /// sRGB with linear alpha
    Srgb = 0,
    /// All channels are linear
    Linear = 1,
}

impl ColorSpace {
    pub const fn is_srgb(self) -> bool {
        matches!(self, Self::Srgb)
    }

    pub const fn is_linear(self) -> bool {
        matches!(self, Self::Linear)
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
