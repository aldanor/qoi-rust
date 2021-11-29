use crate::colorspace::ColorSpace;
use crate::encode::qoi_encode_to_vec_impl;
use crate::error::Result;

pub fn qoi_encode_to_vec(
    data: impl AsRef<[u8]>, width: u32, height: u32, channels: u8,
    colorspace: impl Into<ColorSpace>,
) -> Result<Vec<u8>> {
    qoi_encode_to_vec_impl::<true>(data.as_ref(), width, height, channels, colorspace.into())
}
