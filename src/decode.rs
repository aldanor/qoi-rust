use std::mem;

use crate::consts::{
    QOI_HEADER_SIZE, QOI_OP_DIFF, QOI_OP_INDEX, QOI_OP_LUMA, QOI_OP_RGB, QOI_OP_RGBA, QOI_OP_RUN,
    QOI_PADDING_SIZE,
};
use crate::error::{Error, Result};
use crate::header::Header;
use crate::pixel::{Pixel, SupportedChannels};
use crate::utils::unlikely;

struct ReadBuf {
    current: *const u8,
    end: *const u8,
}

impl ReadBuf {
    pub unsafe fn new(ptr: *const u8, len: usize) -> Self {
        Self { current: ptr, end: ptr.add(len) }
    }

    #[inline]
    pub fn read(&mut self) -> u8 {
        unsafe {
            let v = self.current.read();
            self.current = self.current.add(1);
            v
        }
    }

    #[inline]
    pub fn read_array<const N: usize>(&mut self) -> [u8; N] {
        unsafe {
            let v = self.current.cast::<[u8; N]>().read();
            self.current = self.current.add(N);
            v
        }
    }

    #[inline]
    pub fn within_bounds(&self) -> bool {
        self.current < self.end
    }
}

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

    let mut pixels = Vec::<Pixel<N>>::with_capacity(n_pixels);
    unsafe {
        // Safety: we have just allocated enough memory to set the length without problems
        // We will also fill the entire array, and the data type is pod, so there's no UB.
        pixels.set_len(n_pixels);
    }
    let mut buf = unsafe {
        // Safety: we will check within the loop that there are no reads outside the slice
        // (note that QOI_PADDING_SIZE covers all possible read options within a single op)
        ReadBuf::new(data.as_ptr().add(QOI_HEADER_SIZE), data.len() - QOI_HEADER_SIZE)
    };

    let mut index = [Pixel::new(); 64];
    let mut px = Pixel::new().with_a(0xff);
    let mut run = 0_u8;

    for px_out in &mut pixels {
        if run != 0 {
            run -= 1;
            *px_out = px;
            continue;
        } else if unlikely(!buf.within_bounds()) {
            return Err(Error::UnexpectedBufferEnd);
        }

        const QOI_OP_INDEX_END: u8 = QOI_OP_INDEX | 0x3f;
        const QOI_OP_RUN_END: u8 = QOI_OP_RUN | 0x3d; // <- note, 0x3d (not 0x3f)
        const QOI_OP_DIFF_END: u8 = QOI_OP_DIFF | 0x3f;
        const QOI_OP_LUMA_END: u8 = QOI_OP_LUMA | 0x3f;

        match buf.read() {
            b1 @ QOI_OP_INDEX..=QOI_OP_INDEX_END => {
                px = unsafe {
                    // Safety: (b1 ^ QOI_INDEX) is guaranteed to be at most 6 bits
                    *index.get_unchecked(usize::from(b1 ^ QOI_OP_INDEX))
                };
                *px_out = px;
                continue;
            }
            QOI_OP_RGB => {
                px = Pixel::from_rgb(Pixel::from_array(buf.read_array::<3>()), px.a_or(0xff));
            }
            QOI_OP_RGBA => {
                px = Pixel::from_array(buf.read_array::<4>());
            }
            b1 @ QOI_OP_RUN..=QOI_OP_RUN_END => {
                run = b1 & 0x3f;
                *px_out = px;
                continue;
            }
            b1 @ QOI_OP_DIFF..=QOI_OP_DIFF_END => {
                px.rgb_add(
                    ((b1 >> 4) & 0x03).wrapping_sub(2),
                    ((b1 >> 2) & 0x03).wrapping_sub(2),
                    (b1 & 0x03).wrapping_sub(2),
                );
            }
            b1 @ QOI_OP_LUMA..=QOI_OP_LUMA_END => {
                let b2 = buf.read();
                let vg = (b1 & 0x3f).wrapping_sub(32);
                let vg_8 = vg.wrapping_sub(8);
                let vr = vg_8.wrapping_add((b2 >> 4) & 0x0f);
                let vb = vg_8.wrapping_add(b2 & 0x0f);
                px.rgb_add(vr, vg, vb);
            }
        };

        unsafe {
            // Safety: hash_index() is computed mod 64, so it will never go out of bounds
            *index.get_unchecked_mut(usize::from(px.hash_index())) = px;
        }
        *px_out = px;
    }

    let bytes = unsafe {
        // Safety: this is safe because we have previously set all the lengths ourselves
        let ptr = pixels.as_mut_ptr();
        mem::forget(pixels);
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
