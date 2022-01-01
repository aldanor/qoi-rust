// TODO: can be removed once https://github.com/rust-lang/rust/issues/74985 is stable
use bytemuck::{cast_slice, cast_slice_mut, Pod};

use crate::consts::{
    QOI_HEADER_SIZE, QOI_OP_DIFF, QOI_OP_INDEX, QOI_OP_LUMA, QOI_OP_RGB, QOI_OP_RGBA, QOI_OP_RUN,
    QOI_PADDING, QOI_PADDING_SIZE,
};
use crate::error::{Error, Result};
use crate::header::Header;
use crate::pixel::{Pixel, SupportedChannels};
use crate::utils::{cold, unlikely};

const QOI_OP_INDEX_END: u8 = QOI_OP_INDEX | 0x3f;
const QOI_OP_RUN_END: u8 = QOI_OP_RUN | 0x3d; // <- note, 0x3d (not 0x3f)
const QOI_OP_DIFF_END: u8 = QOI_OP_DIFF | 0x3f;
const QOI_OP_LUMA_END: u8 = QOI_OP_LUMA | 0x3f;

#[inline(always)]
pub const fn hash_pixel<const N: usize>(px: [u8; N]) -> u8 {
    let r = px[0].wrapping_mul(3);
    let g = px[1].wrapping_mul(5);
    let b = px[2].wrapping_mul(7);
    let a = (if N == 4 { px[3] } else { 0xff }).wrapping_mul(11);
    r.wrapping_add(g).wrapping_add(b).wrapping_add(a) & 0x3f
}

macro_rules! decode {
    (rgb: $r:expr, $g:expr, $b:expr => $px:expr) => {
        $px[0] = $r;
        $px[1] = $g;
        $px[2] = $b;
    };
    (diff: $b1:expr => $px:expr) => {
        $px[0] = $px[0].wrapping_add(($b1 >> 4) & 0x03).wrapping_sub(2);
        $px[1] = $px[1].wrapping_add(($b1 >> 2) & 0x03).wrapping_sub(2);
        $px[2] = $px[2].wrapping_add($b1 & 0x03).wrapping_sub(2);
    };
    (luma: $b1:expr, $b2:expr => $px:expr) => {
        let vg = ($b1 & 0x3f).wrapping_sub(32);
        let vg_8 = vg.wrapping_sub(8);
        let vr = vg_8.wrapping_add(($b2 >> 4) & 0x0f);
        let vb = vg_8.wrapping_add($b2 & 0x0f);
        $px[0] = $px[0].wrapping_add(vr);
        $px[1] = $px[1].wrapping_add(vg);
        $px[2] = $px[2].wrapping_add(vb);
    };
}

#[inline]
fn qoi_decode_impl_slice<const N: usize, const RGBA: bool>(
    data: &[u8], out: &mut [u8],
) -> Result<usize>
where
    Pixel<N>: SupportedChannels,
    [u8; N]: Pod,
{
    let mut pixels = cast_slice_mut::<_, [u8; N]>(out);
    let data_len = data.len();
    let mut data = data;

    let mut index = [[0_u8; N]; 256];
    let mut px = [0_u8; N];
    if N == 4 {
        px[3] = 0xff;
    }

    while let [px_out, ptail @ ..] = pixels {
        pixels = ptail;
        match data {
            [b1 @ QOI_OP_INDEX..=QOI_OP_INDEX_END, dtail @ ..] => {
                px = index[*b1 as usize];
                *px_out = px;
                data = dtail;
                continue;
            }
            [QOI_OP_RGB, r, g, b, dtail @ ..] => {
                decode!(rgb: *r, *g, *b => px);
                data = dtail;
            }
            [QOI_OP_RGBA, r, g, b, a, dtail @ ..] if RGBA => {
                decode!(rgb: *r, *g, *b => px);
                if N == 4 {
                    px[3] = *a;
                }
                data = dtail;
            }
            [b1 @ QOI_OP_RUN..=QOI_OP_RUN_END, dtail @ ..] => {
                *px_out = px;
                let run = ((b1 & 0x3f) as usize).min(pixels.len());
                let (phead, ptail) = pixels.split_at_mut(run); // can't panic
                phead.fill(px);
                pixels = ptail;
                data = dtail;
                continue;
            }
            [b1 @ QOI_OP_DIFF..=QOI_OP_DIFF_END, dtail @ ..] => {
                decode!(diff: b1 => px);
                data = dtail;
            }
            [b1 @ QOI_OP_LUMA..=QOI_OP_LUMA_END, b2, dtail @ ..] => {
                decode!(luma: b1, b2 => px);
                data = dtail;
            }
            _ => {
                cold();
                if unlikely(data.len() < QOI_PADDING_SIZE) {
                    return Err(Error::UnexpectedBufferEnd); // TODO: remove InputDataSize err
                }
            }
        }

        index[hash_pixel(px) as usize] = px;
        *px_out = px;
    }

    if unlikely(data.len() < QOI_PADDING_SIZE) {
        return Err(Error::UnexpectedBufferEnd);
    } else if unlikely(cast_slice::<_, [u8; QOI_PADDING_SIZE]>(data)[0] != QOI_PADDING) {
        return Err(Error::InvalidPadding);
    }

    Ok(data_len.saturating_sub(data.len()).saturating_sub(QOI_PADDING_SIZE))
}

#[inline]
fn qoi_decode_impl_slice_all(
    data: &[u8], out: &mut [u8], channels: u8, src_channels: u8,
) -> Result<usize> {
    match (channels, src_channels) {
        (3, 3) => qoi_decode_impl_slice::<3, false>(data, out),
        (3, 4) => qoi_decode_impl_slice::<3, true>(data, out),
        (4, 3) => qoi_decode_impl_slice::<4, false>(data, out),
        (4, 4) => qoi_decode_impl_slice::<4, true>(data, out),
        _ => {
            cold();
            return Err(Error::InvalidChannels { channels });
        }
    }
}

#[inline]
pub fn qoi_decode_to_buf(buf: impl AsMut<[u8]>, data: impl AsRef<[u8]>) -> Result<Header> {
    let mut decoder = QoiDecoder::new(&data)?;
    decoder.decode_to_buf(buf)?;
    Ok(*decoder.header())
}

#[inline]
pub fn qoi_decode_to_vec(data: impl AsRef<[u8]>) -> Result<(Header, Vec<u8>)> {
    let mut decoder = QoiDecoder::new(&data)?;
    let out = decoder.decode_to_vec()?;
    Ok((*decoder.header(), out))
}

#[inline]
pub fn qoi_decode_header(data: impl AsRef<[u8]>) -> Result<Header> {
    Header::decode(data)
}

#[derive(Clone)]
pub struct QoiDecoder<'a> {
    data: &'a [u8],
    header: Header,
    channels: u8,
}

impl<'a> QoiDecoder<'a> {
    #[inline]
    pub fn new(data: &'a (impl AsRef<[u8]> + ?Sized)) -> Result<Self> {
        let data = data.as_ref();
        let header = Header::decode(data)?;
        let data = &data[QOI_HEADER_SIZE..]; // can't panic
        Ok(Self { data, header, channels: header.channels })
    }

    #[inline]
    pub fn with_channels(mut self, channels: u8) -> Self {
        self.channels = channels;
        self
    }

    #[inline]
    pub fn channels(&self) -> u8 {
        self.channels
    }

    #[inline]
    pub fn header(&self) -> &Header {
        &self.header
    }

    #[inline]
    pub fn data(self) -> &'a [u8] {
        self.data
    }

    #[inline]
    pub fn decode_to_buf(&mut self, mut buf: impl AsMut<[u8]>) -> Result<()> {
        let buf = buf.as_mut();
        let size = self.header.n_pixels() * self.channels as usize;
        if unlikely(buf.len() < size) {
            return Err(Error::OutputBufferTooSmall { size: buf.len(), required: size });
        }
        let n_read =
            qoi_decode_impl_slice_all(self.data, buf, self.channels, self.header.channels)?;
        self.data = &self.data[n_read..]; // can't panic
        Ok(())
    }

    #[inline]
    pub fn decode_to_vec(&mut self) -> Result<Vec<u8>> {
        if unlikely(self.channels > 4) {
            // prevent accidental over-allocations
            cold();
            return Err(Error::InvalidChannels { channels: self.channels });
        }
        let mut out = vec![0; self.header.n_pixels() * self.channels as usize];
        self.decode_to_buf(&mut out).map(|_| out)
    }
}
