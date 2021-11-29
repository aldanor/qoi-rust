use std::mem;

use crate::consts::{
    QOI_COLOR, QOI_DIFF_16, QOI_DIFF_24, QOI_DIFF_8, QOI_HEADER_SIZE, QOI_INDEX, QOI_MAGIC,
    QOI_MASK_2, QOI_MASK_3, QOI_MASK_4, QOI_PADDING, QOI_RUN_16, QOI_RUN_8,
};
use crate::error::{Error, Result};
use crate::header::Header;
use crate::pixel::{Pixel, SupportedChannels};

struct ReadBuf {
    start: *const u8,
    end: *const u8,
}

impl ReadBuf {
    pub unsafe fn new(ptr: *const u8) -> Self {
        Self { start: ptr, end: ptr }
    }

    #[inline]
    pub fn read(&mut self) -> u8 {
        unsafe {
            let v = self.end.read();
            self.end = self.end.add(1);
            v
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        unsafe { self.end.offset_from(self.start).max(0) as usize }
    }
}

pub fn qoi_decode_impl<const N: usize>(data: &[u8]) -> Result<(Header, Vec<u8>)>
where
    Pixel<N>: SupportedChannels,
{
    if data.len() < QOI_HEADER_SIZE + QOI_PADDING {
        return Err(Error::BadDecodingDataSize { size: data.len() });
    }
    let header = Header::from_bytes(unsafe {
        // Safety: Header is a POD type and we have just checked that the data fits it
        *(data.as_ptr() as *const _)
    });

    let n_pixels = (header.width as usize) * (header.height as usize);
    if n_pixels == 0 {
        return Err(Error::EmptyImage { width: header.width, height: header.height });
    }
    if header.magic != QOI_MAGIC {
        return Err(Error::InvalidMagic { magic: header.magic });
    }

    let mut pixels = Vec::<Pixel<N>>::with_capacity(n_pixels);
    unsafe {
        // Safety: we have just allocated enough memory to set the length without problems
        pixels.set_len(n_pixels);
    }
    let mut buf = unsafe {
        // Safety: we will check within the loop that there are no reads outside the slice
        ReadBuf::new(data.as_ptr().add(QOI_HEADER_SIZE))
    };

    let mut index = [Pixel::new(); 64];
    let mut px = Pixel::new().with_a(0xff);
    let mut run = 0_u16;

    for px_out in pixels.iter_mut() {
        // TODO: check for safety that ReadBuf is not over yet
        if run != 0 {
            run -= 1;
            *px_out = px;
            continue;
        }

        let b1 = buf.read();
        match b1 >> 4 {
            0..=3 => {
                // QOI_INDEX
                px = unsafe {
                    // Safety: (b1 ^ QOI_INDEX) is guaranteed to be at most 6 bits
                    *index.get_unchecked(usize::from(b1 ^ QOI_INDEX))
                };
            }
            15 => {
                // QOI_COLOR
                if b1 & 8 != 0 {
                    px.set_r(buf.read());
                }
                if b1 & 4 != 0 {
                    px.set_g(buf.read());
                }
                if b1 & 2 != 0 {
                    px.set_b(buf.read());
                }
                if b1 & 1 != 0 {
                    px.set_a(buf.read());
                }
            }
            12..=13 => {
                // QOI_DIFF_16
                let b2 = buf.read();
                px.rgb_add(
                    (b1 & 0x1f).wrapping_sub(16),
                    (b2 >> 4).wrapping_sub(8),
                    (b2 & 0x0f).wrapping_sub(8),
                );
            }
            14 => {
                // QOI_DIFF_24
                let (b2, b3) = (buf.read(), buf.read());
                px.rgba_add(
                    (((b1 & 0x0f) << 1) | (b2 >> 7)).wrapping_sub(16),
                    ((b2 & 0x7c) >> 2).wrapping_sub(16),
                    (((b2 & 0x03) << 3) | ((b3 & 0xe0) >> 5)).wrapping_sub(16),
                    (b3 & 0x1f).wrapping_sub(16),
                );
            }
            4..=5 => {
                // QOI_RUN_8
                run = u16::from(b1 & 0x1f);
                *px_out = px;
                continue;
            }
            8..=11 => {
                // QOI_DIFF_8
                px.rgb_add(
                    ((b1 >> 4) & 0x03).wrapping_sub(2),
                    ((b1 >> 2) & 0x03).wrapping_sub(2),
                    (b1 & 0x03).wrapping_sub(2),
                );
            }
            6..=7 => {
                // QOI_RUN_16
                run = 32 + ((u16::from(b1 & 0x1f) << 8) | u16::from(buf.read()));
                *px_out = px;
                continue;
            }
            _ => {}
        }

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
        Vec::from_raw_parts(ptr as *mut _, n_pixels * N, n_pixels * N)
    };

    Ok((header, bytes))
}

pub fn qoi_decode_to_vec(data: impl AsRef<[u8]>, channels: u8) -> Result<(Header, Vec<u8>)> {
    let data = data.as_ref();
    match channels {
        3 => qoi_decode_impl::<3>(data),
        4 => qoi_decode_impl::<4>(data),
        _ => Err(Error::InvalidChannels { channels }),
    }
}
