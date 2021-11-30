use crate::colorspace::ColorSpace;
use crate::encode::{encode_to_buf_impl, encode_to_vec_impl};
use crate::error::Result;

pub fn qoi_encode_to_vec(
    data: impl AsRef<[u8]>, width: u32, height: u32, channels: u8,
    colorspace: impl Into<ColorSpace>,
) -> Result<Vec<u8>> {
    encode_to_vec_impl::<true>(data.as_ref(), width, height, channels, colorspace.into())
}

pub fn qoi_encode_to_buf(
    mut out: impl AsMut<[u8]>, data: impl AsRef<[u8]>, width: u32, height: u32, channels: u8,
    colorspace: impl Into<ColorSpace>,
) -> Result<usize> {
    encode_to_buf_impl::<true>(
        out.as_mut(),
        data.as_ref(),
        width,
        height,
        channels,
        colorspace.into(),
    )
}
