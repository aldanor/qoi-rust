use std::convert::TryInto;

use crate::colorspace::ColorSpace;
use crate::consts::{
    QOI_HEADER_SIZE, QOI_OP_INDEX, QOI_OP_RUN, QOI_PADDING, QOI_PADDING_SIZE, QOI_PIXELS_MAX,
};
use crate::error::{Error, Result};
use crate::header::Header;
use crate::pixel::{Pixel, SupportedChannels};
use crate::utils::{unlikely, BytesMut};

#[allow(clippy::cast_possible_truncation)]
fn qoi_encode_impl<const N: usize>(
    out: &mut [u8], data: &[u8], width: u32, height: u32, colorspace: ColorSpace,
) -> Result<usize>
where
    Pixel<N>: SupportedChannels,
{
    let max_len = encode_size_required(width, height, N as u8);
    if unlikely(out.len() < max_len) {
        return Err(Error::OutputBufferTooSmall { size: out.len(), required: max_len });
    }

    let n_pixels = (width as usize) * (height as usize);
    if unlikely(data.is_empty()) {
        return Err(Error::EmptyImage { width, height });
    } else if unlikely(n_pixels > QOI_PIXELS_MAX) {
        return Err(Error::ImageTooLarge { width, height });
    } else if unlikely(n_pixels * N != data.len()) {
        return Err(Error::BadEncodingDataSize { size: data.len(), expected: n_pixels * N });
    }

    let out_size = out.len();
    let mut buf = BytesMut::new(out);

    let header = Header { width, height, channels: N as u8, colorspace };
    buf = buf.write_many(&header.encode());

    let mut index = [Pixel::new(); 256];
    let mut px_prev = Pixel::new().with_a(0xff);
    let mut run = 0_u8;
    let mut px = Pixel::<N>::new().with_a(0xff);

    for (i, chunk) in data.chunks_exact(N).enumerate() {
        px.read(chunk);
        if px == px_prev {
            run += 1;
            if run == 62 || unlikely(i == n_pixels - 1) {
                buf = buf.write_one(QOI_OP_RUN | (run - 1));
                run = 0;
            }
        } else {
            if run != 0 {
                buf = buf.write_one(QOI_OP_RUN | (run - 1));
                run = 0;
            }
            let index_pos = px.hash_index();
            let index_px = &mut index[index_pos as usize];
            let px_rgba = px.as_rgba(0xff);
            if *index_px == px_rgba {
                buf = buf.write_one(QOI_OP_INDEX | index_pos);
            } else {
                *index_px = px_rgba;
                buf = px.encode_into(px_prev, buf);
            }
            px_prev = px;
        }
    }

    buf = buf.write_many(&QOI_PADDING);
    Ok(out_size.saturating_sub(buf.len()))
}

#[inline]
pub fn qoi_encode_to_buf<O, D, C>(
    mut out: O, data: D, width: u32, height: u32, channels: u8, colorspace: C,
) -> Result<usize>
where
    O: AsMut<[u8]>,
    D: AsRef<[u8]>,
    C: TryInto<ColorSpace>,
    Error: From<C::Error>,
{
    let out = out.as_mut();
    let data = data.as_ref();
    let colorspace = colorspace.try_into()?;
    match channels {
        3 => qoi_encode_impl::<3>(out, data, width, height, colorspace),
        4 => qoi_encode_impl::<4>(out, data, width, height, colorspace),
        _ => Err(Error::InvalidChannels { channels }),
    }
}

#[inline]
pub fn qoi_encode_to_vec<D, C>(
    data: D, width: u32, height: u32, channels: u8, colorspace: C,
) -> Result<Vec<u8>>
where
    D: AsRef<[u8]>,
    C: TryInto<ColorSpace>,
    Error: From<C::Error>,
{
    let size = encode_size_required(width, height, channels);
    let mut out = vec![0; size]; // note: we could save time here but that won't be safe anymore
    let size = qoi_encode_to_buf(&mut out, data, width, height, channels, colorspace)?;
    out.truncate(size);
    Ok(out)
}

#[inline]
pub fn encode_size_required(width: u32, height: u32, channels: u8) -> usize {
    let (width, height) = (width as usize, height as usize);
    let n_pixels = width.saturating_mul(height);
    QOI_HEADER_SIZE + n_pixels.saturating_mul(usize::from(channels)) + n_pixels + QOI_PADDING_SIZE
}
