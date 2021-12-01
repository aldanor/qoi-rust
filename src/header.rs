use crate::colorspace::ColorSpace;
use crate::consts::{QOI_HEADER_SIZE, QOI_MAGIC};
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

const fn u32_to_be(v: u32) -> [u8; 4] {
    [
        ((0xff000000 & v) >> 24) as u8,
        ((0xff0000 & v) >> 16) as u8,
        ((0xff00 & v) >> 8) as u8,
        (0xff & v) as u8,
    ]
}

const fn u32_from_be(v: &[u8]) -> u32 {
    ((v[0] as u32) << 24) | ((v[1] as u32) << 16) | ((v[2] as u32) << 8) | (v[3] as u32)
}

impl Header {
    pub const SIZE: usize = QOI_HEADER_SIZE;

    pub(crate) fn to_bytes(&self) -> [u8; QOI_HEADER_SIZE] {
        let mut out = [0; QOI_HEADER_SIZE];
        out[..4].copy_from_slice(&u32_to_be(self.magic));
        out[4..8].copy_from_slice(&u32_to_be(self.width));
        out[8..12].copy_from_slice(&u32_to_be(self.height));
        out[12] = self.channels;
        out[13] = self.colorspace.into();
        out
    }

    pub(crate) fn from_bytes(v: [u8; QOI_HEADER_SIZE]) -> Self {
        let mut out = Self::default();
        out.magic = u32_from_be(&v[..4]);
        out.width = u32_from_be(&v[4..8]);
        out.height = u32_from_be(&v[8..12]);
        out.channels = v[12];
        out.colorspace = v[13].into();
        out
    }

    pub const fn n_pixels(&self) -> usize {
        (self.width as usize).saturating_mul(self.height as usize)
    }

    #[inline]
    pub fn validate(&self) -> Result<()> {
        if unlikely(self.magic != QOI_MAGIC) {
            return Err(Error::InvalidMagic { magic: self.magic });
        } else if unlikely(self.height == 0 || self.width == 0) {
            return Err(Error::EmptyImage { width: self.width, height: self.height });
        } else if unlikely(self.channels < 3 || self.channels > 4) {
            return Err(Error::InvalidChannels { channels: self.channels });
        }
        Ok(())
    }
}
