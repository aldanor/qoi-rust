use crate::colorspace::ColorSpace;
use crate::consts::{
    QOI_HEADER_SIZE, QOI_OP_DIFF, QOI_OP_INDEX, QOI_OP_LUMA, QOI_OP_RGB, QOI_OP_RGBA, QOI_OP_RUN,
    QOI_PADDING, QOI_PADDING_SIZE, QOI_PIXELS_MAX,
};
use crate::error::{Error, Result};
use crate::header::Header;
use crate::pixel::{Pixel, SupportedChannels};
use crate::utils::{cold, unlikely};

struct WriteBuf<'a> {
    buf: &'a mut [u8],
}

impl<'a> WriteBuf<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf }
    }

    #[inline]
    pub fn write<const N: usize>(self, v: [u8; N]) -> Self {
        let (head, tail) = self.buf.split_at_mut(N);
        head.copy_from_slice(&v);
        Self { buf: tail }
    }

    #[inline]
    pub fn push(self, v: u8) -> Self {
        if let Some((first, tail)) = self.buf.split_first_mut() {
            *first = v;
            Self { buf: tail }
        } else {
            cold();
            panic!();
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.buf.len()
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

    let out_size = out.len();
    let mut buf = WriteBuf::new(out);

    let header =
        Header { width, height, channels: CHANNELS as u8, colorspace, ..Header::default() };
    buf = buf.write(header.to_bytes());

    let mut index = [Pixel::new(); 256];
    let mut px_prev = Pixel::new().with_a(0xff);
    let mut run = 0_u8;
    let mut px = Pixel::<CHANNELS>::new().with_a(0xff);

    for (i, chunk) in data.chunks_exact(CHANNELS).enumerate() {
        px.read(chunk);
        if px == px_prev {
            run += 1;
            if run == 62 || unlikely(i == n_pixels - 1) {
                buf = buf.push(QOI_OP_RUN | (run - 1));
                run = 0;
            }
        } else {
            if run != 0 {
                buf = buf.push(QOI_OP_RUN | (run - 1));
                run = 0;
            }
            let index_pos = px.hash_index();
            let index_px = &mut index[usize::from(index_pos)];
            let px_rgba = px.as_rgba(0xff);
            if *index_px == px_rgba {
                buf = buf.push(QOI_OP_INDEX | index_pos);
            } else {
                *index_px = px_rgba;

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
                        buf = buf.push(QOI_OP_DIFF | vr_2 << 4 | vg_2 << 2 | vb_2);
                    } else {
                        let (vg_32, vg_r_8, vg_b_8) =
                            (vg.wrapping_add(32), vg_r.wrapping_add(8), vg_b.wrapping_add(8));
                        if vg_r_8 | vg_b_8 | 15 == 15 && vg_32 | 63 == 63 {
                            buf = buf.write([QOI_OP_LUMA | vg_32, vg_r_8 << 4 | vg_b_8]);
                        } else {
                            buf = buf.write([QOI_OP_RGB, px.r(), px.g(), px.b()]);
                        }
                    }
                } else {
                    buf = buf.write([QOI_OP_RGBA, px.r(), px.g(), px.b(), px.a_or(0xff)]);
                }
            }
            px_prev = px;
        }
    }

    buf = buf.write(QOI_PADDING);
    Ok(out_size.saturating_sub(buf.len()))
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
