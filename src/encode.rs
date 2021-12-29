use std::slice;

use crate::colorspace::ColorSpace;
use crate::consts::{
    QOI_HEADER_SIZE, QOI_OP_DIFF, QOI_OP_INDEX, QOI_OP_LUMA, QOI_OP_RGB, QOI_OP_RGBA, QOI_OP_RUN,
    QOI_PADDING, QOI_PADDING_SIZE, QOI_PIXELS_MAX,
};
use crate::error::{Error, Result};
use crate::header::Header;
use crate::pixel::{Pixel, SupportedChannels};
use crate::utils::unlikely;

struct WriteBuf {
    start: *const u8,
    current: *mut u8,
}

impl WriteBuf {
    pub const unsafe fn new(ptr: *mut u8) -> Self {
        Self { start: ptr, current: ptr }
    }

    #[inline]
    pub fn write<const N: usize>(&mut self, v: [u8; N]) {
        unsafe {
            // TODO: single write via deref?
            let mut i = 0;
            while i < N {
                self.current.add(i).write(v[i]);
                i += 1;
            }
            self.current = self.current.add(N);
        }
    }

    #[inline]
    pub fn push(&mut self, v: u8) {
        unsafe {
            self.current.write(v);
            self.current = self.current.add(1);
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        unsafe { self.current.offset_from(self.start).max(0) as usize }
    }
}

fn qoi_encode_impl<const CHANNELS: usize>(
    out: &mut [u8], data: &[u8], width: u32, height: u32, colorspace: ColorSpace,
) -> Result<usize>
where
    Pixel<CHANNELS>: SupportedChannels,
{
    let max_len = encode_size_required(width, height, CHANNELS as u8);
    if unlikely(out.len() < max_len) {
        return Err(Error::OutputBufferTooSmall { size: out.len(), required: max_len });
    }

    let n_pixels = (width as usize) * (height as usize);
    if unlikely(data.is_empty()) {
        return Err(Error::EmptyImage { width, height });
    } else if unlikely(n_pixels > QOI_PIXELS_MAX) {
        return Err(Error::ImageTooLarge { width, height });
    } else if unlikely(n_pixels * CHANNELS != data.len()) {
        return Err(Error::BadEncodingDataSize { size: data.len(), expected: n_pixels * CHANNELS });
    }

    let pixels = unsafe {
        // Safety: we've verified that n_pixels * N == data.len()
        slice::from_raw_parts::<Pixel<CHANNELS>>(data.as_ptr().cast(), n_pixels)
    };

    let mut buf = unsafe {
        // Safety: all write ops are guaranteed to not go outside allocation
        WriteBuf::new(out.as_mut_ptr())
    };

    let header =
        Header { width, height, channels: CHANNELS as u8, colorspace, ..Header::default() };
    buf.write(header.to_bytes());

    let mut index = [Pixel::new(); 64];
    let mut px_prev = Pixel::new().with_a(0xff);
    let mut run = 0_u8;

    for (i, &px) in pixels.iter().enumerate() {
        if px == px_prev {
            run += 1;
            if run == 62 || unlikely(i == n_pixels - 1) {
                buf.push(QOI_OP_RUN | (run - 1));
                run = 0;
            }
        } else {
            if run != 0 {
                buf.push(QOI_OP_RUN | (run - 1));
                run = 0;
            }
            let index_pos = px.hash_index();
            let index_px = unsafe {
                // Safety: hash_index() is computed mod 64, so it will never go out of bounds
                index.get_unchecked_mut(usize::from(index_pos))
            };
            let px4 = px.as_rgba(0xff);
            if *index_px == px4 {
                buf.push(QOI_OP_INDEX | index_pos);
            } else {
                *index_px = px4;

                if px.a_or(0) == px_prev.a_or(0) {
                    let vr = px.r().wrapping_sub(px_prev.r());
                    let vg = px.g().wrapping_sub(px_prev.g());
                    let vb = px.b().wrapping_sub(px_prev.b());

                    let vg_r = vr.wrapping_sub(vg);
                    let vg_b = vb.wrapping_sub(vg);

                    // TODO maybe add an outer check for vg_32
                    let (vr_2, vg_2, vb_2) =
                        (vr.wrapping_add(2), vg.wrapping_add(2), vb.wrapping_add(2));
                    if vr_2 | vg_2 | vb_2 | 3 == 3 {
                        buf.push(QOI_OP_DIFF | vr_2 << 4 | vg_2 << 2 | vb_2);
                    } else {
                        let (vg_32, vg_r_8, vg_b_8) =
                            (vg.wrapping_add(32), vg_r.wrapping_add(8), vg_b.wrapping_add(8));
                        if vg_r_8 | vg_b_8 | 15 == 15 && vg_32 | 63 == 63 {
                            buf.write([QOI_OP_LUMA | vg_32, vg_r_8 << 4 | vg_b_8]);
                        } else {
                            buf.write([QOI_OP_RGB, px.r(), px.g(), px.b()]);
                        }
                    }
                } else {
                    // TODO: or 2 write ops? (QOI_OP_RGBA and px.into_array())
                    buf.write([QOI_OP_RGBA, px.r(), px.g(), px.b(), px.a_or(0xff)]);
                }
            }
            px_prev = px;
        }
    }

    buf.write(QOI_PADDING);
    Ok(buf.len())
}

#[inline]
pub fn qoi_encode_to_buf(
    mut out: impl AsMut<[u8]>, data: impl AsRef<[u8]>, width: u32, height: u32, channels: u8,
    colorspace: impl Into<ColorSpace>,
) -> Result<usize> {
    let out = out.as_mut();
    let data = data.as_ref();
    let colorspace = colorspace.into();
    match channels {
        3 => qoi_encode_impl::<3>(out, data, width, height, colorspace),
        4 => qoi_encode_impl::<4>(out, data, width, height, colorspace),
        _ => Err(Error::InvalidChannels { channels }),
    }
}

#[inline]
pub fn qoi_encode_to_vec(
    data: impl AsRef<[u8]>, width: u32, height: u32, channels: u8,
    colorspace: impl Into<ColorSpace>,
) -> Result<Vec<u8>> {
    let data = data.as_ref();
    let colorspace = colorspace.into();
    let mut out = Vec::with_capacity(encode_size_required(width, height, channels));
    unsafe {
        out.set_len(out.capacity());
    }
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
