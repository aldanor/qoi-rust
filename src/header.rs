use crate::colorspace::ColorSpace;
use crate::consts::{QOI_HEADER_SIZE, QOI_MAGIC, QOI_PIXELS_MAX};
use crate::error::{Error, Result};
use crate::utils::unlikely;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub magic: u32,
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub colorspace: ColorSpace,
}

impl Default for Header {
    #[inline]
    fn default() -> Self {
        Self {
            magic: QOI_MAGIC,
            width: 1,
            height: 1,
            channels: 3,
            colorspace: ColorSpace::default(),
        }
    }
}

#[inline(always)]
const fn u32_to_be(v: u32) -> [u8; 4] {
    [
        ((0xff00_0000 & v) >> 24) as u8,
        ((0x00ff_0000 & v) >> 16) as u8,
        ((0xff00 & v) >> 8) as u8,
        (0x00ff & v) as u8,
    ]
}

#[inline(always)]
const fn u32_from_be(v: &[u8]) -> u32 {
    ((v[0] as u32) << 24) | ((v[1] as u32) << 16) | ((v[2] as u32) << 8) | (v[3] as u32)
}

impl Header {
    #[inline]
    pub const fn new(width: u32, height: u32, channels: u8) -> Self {
        Self { magic: QOI_MAGIC, width, height, channels, colorspace: ColorSpace::from_u8(0) }
    }

    #[inline]
    pub fn with_colorspace(mut self, colorspace: impl Into<ColorSpace>) -> Self {
        self.colorspace = colorspace.into();
        self
    }

    #[inline]
    pub const fn encoded_size() -> usize {
        QOI_HEADER_SIZE
    }

    #[inline]
    pub(crate) fn to_bytes(&self) -> [u8; QOI_HEADER_SIZE] {
        let mut out = [0; QOI_HEADER_SIZE];
        out[..4].copy_from_slice(&u32_to_be(self.magic));
        out[4..8].copy_from_slice(&u32_to_be(self.width));
        out[8..12].copy_from_slice(&u32_to_be(self.height));
        out[12] = self.channels;
        out[13] = self.colorspace.into();
        out
    }

    #[inline]
    pub(crate) fn from_bytes(v: [u8; QOI_HEADER_SIZE]) -> Self {
        Self {
            magic: u32_from_be(&v[..4]),
            width: u32_from_be(&v[4..8]),
            height: u32_from_be(&v[8..12]),
            channels: v[12],
            colorspace: v[13].into(),
        }
    }

    #[inline]
    pub const fn n_pixels(&self) -> usize {
        (self.width as usize).saturating_mul(self.height as usize)
    }

    #[inline]
    pub const fn validate(&self) -> Result<()> {
        if unlikely(self.magic != QOI_MAGIC) {
            return Err(Error::InvalidMagic { magic: self.magic });
        } else if unlikely(self.height == 0 || self.width == 0) {
            return Err(Error::EmptyImage { width: self.width, height: self.height });
        } else if unlikely((self.height as usize) * (self.width as usize) > QOI_PIXELS_MAX) {
            return Err(Error::ImageTooLarge { width: self.width, height: self.height });
        } else if unlikely(self.channels < 3 || self.channels > 4) {
            return Err(Error::InvalidChannels { channels: self.channels });
        }
        Ok(())
    }
}
