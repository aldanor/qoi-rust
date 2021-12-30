use std::mem;

use crate::consts::{
    QOI_HEADER_SIZE, QOI_OP_DIFF, QOI_OP_INDEX, QOI_OP_LUMA, QOI_OP_RGB, QOI_OP_RGBA, QOI_OP_RUN,
    QOI_PADDING_SIZE,
};
use crate::error::{Error, Result};
use crate::header::Header;
use crate::pixel::{Pixel, SupportedChannels};
use crate::utils::{cold, likely, unlikely};

pub fn qoi_decode_impl<const N: usize>(data: &[u8], n_pixels: usize) -> Result<Vec<u8>>
where
    Pixel<N>: SupportedChannels,
{
    if unlikely(data.len() < QOI_HEADER_SIZE + QOI_PADDING_SIZE) {
        return Err(Error::InputBufferTooSmall {
            size: data.len(),
            required: QOI_HEADER_SIZE + QOI_PADDING_SIZE,
        });
    }

    let mut pixels = vec![Pixel::<N>::new(); n_pixels];
    let mut index = [Pixel::new(); 256];
    let mut px = Pixel::new().with_a(0xff);

    const QOI_OP_INDEX_END: u8 = QOI_OP_INDEX | 0x3f;
    const QOI_OP_RUN_END: u8 = QOI_OP_RUN | 0x3d; // <- note, 0x3d (not 0x3f)
    const QOI_OP_DIFF_END: u8 = QOI_OP_DIFF | 0x3f;
    const QOI_OP_LUMA_END: u8 = QOI_OP_LUMA | 0x3f;

    {
        let mut pixels = &mut pixels[..];
        let mut data = &data[QOI_HEADER_SIZE..];
        loop {
            match pixels {
                [px_out, tail @ ..] => {
                    pixels = tail;
                    match data {
                        [b1 @ QOI_OP_INDEX..=QOI_OP_INDEX_END, dtail @ ..] => {
                            px = index[usize::from(*b1)];
                            *px_out = px;
                            data = dtail;
                            continue;
                        }
                        [QOI_OP_RGB, r, g, b, dtail @ ..] => {
                            px = Pixel::from_rgb(Pixel::from_array([*r, *g, *b]), px.a_or(0xff));
                            data = dtail;
                        }
                        [QOI_OP_RGBA, r, g, b, a, dtail @ ..] => {
                            if N == 4 {
                                cold();
                            }
                            px = Pixel::from_array([*r, *g, *b, *a]);
                            data = dtail;
                        }
                        [b1 @ QOI_OP_RUN..=QOI_OP_RUN_END, dtail @ ..] => {
                            let run = usize::from(b1 & 0x3f).min(pixels.len());
                            *px_out = px;
                            if likely(run != 0) {
                                let (phead, ptail) = pixels.split_at_mut(run); // can't panic
                                phead.fill(px);
                                pixels = ptail;
                            }
                            data = dtail;
                            continue;
                        }
                        [b1 @ QOI_OP_DIFF..=QOI_OP_DIFF_END, dtail @ ..] => {
                            px.rgb_add(
                                ((b1 >> 4) & 0x03).wrapping_sub(2),
                                ((b1 >> 2) & 0x03).wrapping_sub(2),
                                (b1 & 0x03).wrapping_sub(2),
                            );
                            data = dtail;
                        }
                        [b1 @ QOI_OP_LUMA..=QOI_OP_LUMA_END, b2, dtail @ ..] => {
                            let vg = (b1 & 0x3f).wrapping_sub(32);
                            let vg_8 = vg.wrapping_sub(8);
                            let vr = vg_8.wrapping_add((b2 >> 4) & 0x0f);
                            let vb = vg_8.wrapping_add(b2 & 0x0f);
                            px.rgb_add(vr, vg, vb);
                            data = dtail;
                        }
                        _ => {
                            cold();
                            if unlikely(data.len() < 8) {
                                return Err(Error::UnexpectedBufferEnd);
                            }
                        }
                    }
                    index[usize::from(px.hash_index())] = px;
                    *px_out = px;
                }
                _ => {
                    cold();
                    break;
                }
            }
        }
    }

    let ptr = pixels.as_mut_ptr();
    mem::forget(pixels);
    let bytes = unsafe {
        // Safety: this is safe because we have previously set all the lengths ourselves
        Vec::from_raw_parts(ptr.cast(), n_pixels * N, n_pixels * N)
    };

    Ok(bytes)
}

pub trait MaybeChannels {
    fn maybe_channels(self) -> Option<u8>;
}

impl MaybeChannels for u8 {
    #[inline]
    fn maybe_channels(self) -> Option<u8> {
        Some(self)
    }
}

impl MaybeChannels for Option<u8> {
    #[inline]
    fn maybe_channels(self) -> Option<u8> {
        self
    }
}

#[inline]
pub fn qoi_decode_to_vec(
    data: impl AsRef<[u8]>, channels: impl MaybeChannels,
) -> Result<(Header, Vec<u8>)> {
    let data = data.as_ref();
    let header = qoi_decode_header(data)?;
    header.validate()?;
    let channels = channels.maybe_channels().unwrap_or(header.channels);
    match channels {
        3 => Ok((header, qoi_decode_impl::<3>(data, header.n_pixels())?)),
        4 => Ok((header, qoi_decode_impl::<4>(data, header.n_pixels())?)),
        _ => Err(Error::InvalidChannels { channels }),
    }
}

#[inline]
pub fn qoi_decode_header(data: impl AsRef<[u8]>) -> Result<Header> {
    let data = data.as_ref();
    if unlikely(data.len() < QOI_HEADER_SIZE) {
        return Err(Error::InputBufferTooSmall { size: data.len(), required: QOI_HEADER_SIZE });
    }
    let mut bytes = [0_u8; QOI_HEADER_SIZE];
    bytes.copy_from_slice(&data[..QOI_HEADER_SIZE]);
    Ok(Header::from_bytes(bytes))
}
