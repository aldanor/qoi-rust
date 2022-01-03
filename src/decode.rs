use std::io::Read;

// TODO: can be removed once https://github.com/rust-lang/rust/issues/74985 is stable
use bytemuck::{cast_slice_mut, Pod};

use crate::consts::{
    QOI_HEADER_SIZE, QOI_OP_DIFF, QOI_OP_INDEX, QOI_OP_LUMA, QOI_OP_RGB, QOI_OP_RGBA, QOI_OP_RUN,
    QOI_PADDING, QOI_PADDING_SIZE,
};
use crate::error::{Error, Result};
use crate::header::Header;
use crate::pixel::{Pixel, SupportedChannels};
use crate::types::Channels;
use crate::utils::{cold, unlikely};

const QOI_OP_INDEX_END: u8 = QOI_OP_INDEX | 0x3f;
const QOI_OP_RUN_END: u8 = QOI_OP_RUN | 0x3d; // <- note, 0x3d (not 0x3f)
const QOI_OP_DIFF_END: u8 = QOI_OP_DIFF | 0x3f;
const QOI_OP_LUMA_END: u8 = QOI_OP_LUMA | 0x3f;

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

    let mut index = [Pixel::<N>::new(); 256];
    let mut px = Pixel::<N>::new().with_a(0xff);

    while let [px_out, ptail @ ..] = pixels {
        pixels = ptail;
        match data {
            [b1 @ QOI_OP_INDEX..=QOI_OP_INDEX_END, dtail @ ..] => {
                px = index[*b1 as usize];
                *px_out = px.into();
                data = dtail;
                continue;
            }
            [QOI_OP_RGB, r, g, b, dtail @ ..] => {
                px.update_rgb(*r, *g, *b);
                data = dtail;
            }
            [QOI_OP_RGBA, r, g, b, a, dtail @ ..] if RGBA => {
                px.update_rgba(*r, *g, *b, *a);
                data = dtail;
            }
            [b1 @ QOI_OP_RUN..=QOI_OP_RUN_END, dtail @ ..] => {
                *px_out = px.into();
                let run = ((b1 & 0x3f) as usize).min(pixels.len());
                let (phead, ptail) = pixels.split_at_mut(run); // can't panic
                phead.fill(px.into());
                pixels = ptail;
                data = dtail;
                continue;
            }
            [b1 @ QOI_OP_DIFF..=QOI_OP_DIFF_END, dtail @ ..] => {
                px.update_diff(*b1);
                data = dtail;
            }
            [b1 @ QOI_OP_LUMA..=QOI_OP_LUMA_END, b2, dtail @ ..] => {
                px.update_luma(*b1, *b2);
                data = dtail;
            }
            _ => {
                cold();
                if unlikely(data.len() < QOI_PADDING_SIZE) {
                    return Err(Error::UnexpectedBufferEnd); // TODO: remove InputDataSize err
                }
            }
        }

        index[px.hash_index() as usize] = px;
        *px_out = px.into();
    }

    if unlikely(data.len() < QOI_PADDING_SIZE) {
        return Err(Error::UnexpectedBufferEnd);
    } else if unlikely(data[..QOI_PADDING_SIZE] != QOI_PADDING) {
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
            Err(Error::InvalidChannels { channels })
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
    channels: Channels,
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
    pub const fn with_channels(mut self, channels: Channels) -> Self {
        self.channels = channels;
        self
    }

    #[inline]
    pub const fn channels(&self) -> Channels {
        self.channels
    }

    #[inline]
    pub const fn header(&self) -> &Header {
        &self.header
    }

    #[inline]
    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    #[inline]
    pub fn decode_to_buf(&mut self, mut buf: impl AsMut<[u8]>) -> Result<usize> {
        let buf = buf.as_mut();
        let size = self.header.n_pixels() * self.channels.as_u8() as usize;
        if unlikely(buf.len() < size) {
            return Err(Error::OutputBufferTooSmall { size: buf.len(), required: size });
        }
        let n_read = qoi_decode_impl_slice_all(
            self.data,
            buf,
            self.channels.as_u8(),
            self.header.channels.as_u8(),
        )?;
        self.data = &self.data[n_read..]; // can't panic
        Ok(size)
    }

    #[inline]
    pub fn decode_to_vec(&mut self) -> Result<Vec<u8>> {
        let mut out = vec![0; self.header.n_pixels() * self.channels.as_u8() as usize];
        self.decode_to_buf(&mut out).map(|_| out)
    }
}

#[inline]
fn qoi_decode_impl_stream<R: Read, const N: usize, const RGBA: bool>(
    data: &mut R, out: &mut [u8],
) -> Result<()>
where
    Pixel<N>: SupportedChannels,
    [u8; N]: Pod,
{
    let mut pixels = cast_slice_mut::<_, [u8; N]>(out);

    let mut index = [Pixel::<N>::new(); 256];
    let mut px = Pixel::<N>::new().with_a(0xff);

    while let [px_out, ptail @ ..] = pixels {
        pixels = ptail;
        let mut p = [0];
        data.read_exact(&mut p)?;
        let [b1] = p;
        match b1 {
            QOI_OP_INDEX..=QOI_OP_INDEX_END => {
                px = index[b1 as usize];
                *px_out = px.into();
                continue;
            }
            QOI_OP_RGB => {
                let mut p = [0; 3];
                data.read_exact(&mut p)?;
                px.update_rgb(p[0], p[1], p[2]);
            }
            QOI_OP_RGBA if RGBA => {
                let mut p = [0; 4];
                data.read_exact(&mut p)?;
                px.update_rgba(p[0], p[1], p[2], p[3]);
            }
            QOI_OP_RUN..=QOI_OP_RUN_END => {
                *px_out = px.into();
                let run = ((b1 & 0x3f) as usize).min(pixels.len());
                let (phead, ptail) = pixels.split_at_mut(run); // can't panic
                phead.fill(px.into());
                pixels = ptail;
                continue;
            }
            QOI_OP_DIFF..=QOI_OP_DIFF_END => {
                px.update_diff(b1);
            }
            QOI_OP_LUMA..=QOI_OP_LUMA_END => {
                let mut p = [0];
                data.read_exact(&mut p)?;
                let [b2] = p;
                px.update_luma(b1, b2);
            }
            _ => {
                cold();
            }
        }

        index[px.hash_index() as usize] = px;
        *px_out = px.into();
    }

    let mut p = [0_u8; QOI_PADDING_SIZE];
    data.read_exact(&mut p)?;
    if unlikely(p != QOI_PADDING) {
        return Err(Error::InvalidPadding);
    }

    Ok(())
}

#[inline]
fn qoi_decode_impl_stream_all<R: Read>(
    data: &mut R, out: &mut [u8], channels: u8, src_channels: u8,
) -> Result<()> {
    match (channels, src_channels) {
        (3, 3) => qoi_decode_impl_stream::<_, 3, false>(data, out),
        (3, 4) => qoi_decode_impl_stream::<_, 3, true>(data, out),
        (4, 3) => qoi_decode_impl_stream::<_, 4, false>(data, out),
        (4, 4) => qoi_decode_impl_stream::<_, 4, true>(data, out),
        _ => {
            cold();
            Err(Error::InvalidChannels { channels })
        }
    }
}

pub struct QoiStreamDecoder<R> {
    reader: R,
    header: Header,
    channels: Channels,
}

impl<R: Read> QoiStreamDecoder<R> {
    #[inline]
    pub fn new(mut reader: R) -> Result<Self> {
        let mut b = [0; QOI_HEADER_SIZE];
        reader.read_exact(&mut b)?;
        let header = Header::decode(b)?;
        Ok(Self { reader, header, channels: header.channels })
    }

    pub fn with_channels(mut self, channels: Channels) -> Self {
        self.channels = channels;
        self
    }

    #[inline]
    pub fn channels(&self) -> Channels {
        self.channels
    }

    #[inline]
    pub fn header(&self) -> &Header {
        &self.header
    }

    #[inline]
    pub fn reader(&self) -> &R {
        &self.reader
    }

    #[inline]
    pub fn into_reader(self) -> R {
        self.reader
    }

    #[inline]
    pub fn decode_to_buf(&mut self, mut buf: impl AsMut<[u8]>) -> Result<usize> {
        let buf = buf.as_mut();
        let size = self.header.n_pixels() * self.channels.as_u8() as usize;
        if unlikely(buf.len() < size) {
            return Err(Error::OutputBufferTooSmall { size: buf.len(), required: size });
        }
        qoi_decode_impl_stream_all(
            &mut self.reader,
            buf,
            self.channels.as_u8(),
            self.header.channels.as_u8(),
        )?;
        Ok(size)
    }

    #[inline]
    pub fn decode_to_vec(&mut self) -> Result<Vec<u8>> {
        let mut out = vec![0; self.header.n_pixels() * self.channels.as_u8() as usize];
        let _ = self.decode_to_buf(&mut out)?;
        Ok(out)
    }
}
