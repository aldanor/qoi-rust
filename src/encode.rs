use std::convert::TryFrom;
use std::io::Write;

use crate::consts::{QOI_HEADER_SIZE, QOI_OP_INDEX, QOI_OP_RUN, QOI_PADDING, QOI_PADDING_SIZE};
use crate::error::{Error, Result};
use crate::header::Header;
use crate::pixel::{Pixel, SupportedChannels};
use crate::types::{Channels, ColorSpace};
use crate::utils::{unlikely, BytesMut, GenericWriter, Writer};

#[allow(clippy::cast_possible_truncation)]
fn qoi_encode_impl<W: Writer, const N: usize>(mut buf: W, data: &[u8]) -> Result<usize>
where
    Pixel<N>: SupportedChannels,
{
    let cap = buf.capacity();

    let mut index = [Pixel::new(); 256];
    let mut px_prev = Pixel::new().with_a(0xff);
    let mut run = 0_u8;
    let mut px = Pixel::<N>::new().with_a(0xff);

    let n_pixels = data.len() / N;

    for (i, chunk) in data.chunks_exact(N).enumerate() {
        px.read(chunk);
        if px == px_prev {
            run += 1;
            if run == 62 || unlikely(i == n_pixels - 1) {
                buf = buf.write_one(QOI_OP_RUN | (run - 1))?;
                run = 0;
            }
        } else {
            if run != 0 {
                buf = buf.write_one(QOI_OP_RUN | (run - 1))?;
                run = 0;
            }
            let index_pos = px.hash_index();
            let index_px = &mut index[index_pos as usize];
            let px_rgba = px.as_rgba(0xff);
            if *index_px == px_rgba {
                buf = buf.write_one(QOI_OP_INDEX | index_pos)?;
            } else {
                *index_px = px_rgba;
                buf = px.encode_into(px_prev, buf)?;
            }
            px_prev = px;
        }
    }

    buf = buf.write_many(&QOI_PADDING)?;
    Ok(cap.saturating_sub(buf.capacity()))
}

#[inline]
fn qoi_encode_impl_all<W: Writer>(out: W, data: &[u8], channels: Channels) -> Result<usize> {
    match channels {
        Channels::Rgb => qoi_encode_impl::<_, 3>(out, data),
        Channels::Rgba => qoi_encode_impl::<_, 4>(out, data),
    }
}

#[inline]
pub fn encoded_size_limit(width: u32, height: u32, channels: impl Into<u8>) -> usize {
    let (width, height) = (width as usize, height as usize);
    let n_pixels = width.saturating_mul(height);
    QOI_HEADER_SIZE
        + n_pixels.saturating_mul(channels.into() as usize)
        + n_pixels
        + QOI_PADDING_SIZE
}

#[inline]
pub fn qoi_encode_to_buf(
    buf: impl AsMut<[u8]>, data: impl AsRef<[u8]>, width: u32, height: u32,
) -> Result<usize> {
    QoiEncoder::new(&data, width, height)?.encode_to_buf(buf)
}

#[inline]
pub fn qoi_encode_to_vec(data: impl AsRef<[u8]>, width: u32, height: u32) -> Result<Vec<u8>> {
    QoiEncoder::new(&data, width, height)?.encode_to_vec()
}

pub struct QoiEncoder<'a> {
    data: &'a [u8],
    header: Header,
}

impl<'a> QoiEncoder<'a> {
    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    pub fn new(data: &'a (impl AsRef<[u8]> + ?Sized), width: u32, height: u32) -> Result<Self> {
        let data = data.as_ref();
        let mut header =
            Header::try_new(width, height, Channels::default(), ColorSpace::default())?;
        let size = data.len();
        let n_channels = size / header.n_pixels();
        if header.n_pixels() * n_channels != size {
            return Err(Error::InvalidImageLength { size, width, height });
        }
        header.channels = Channels::try_from(n_channels.min(0xff) as u8)?;
        Ok(Self { data, header })
    }

    #[inline]
    pub const fn with_colorspace(mut self, colorspace: ColorSpace) -> Self {
        self.header = self.header.with_colorspace(colorspace);
        self
    }

    #[inline]
    pub const fn channels(&self) -> Channels {
        self.header.channels
    }

    #[inline]
    pub const fn header(&self) -> &Header {
        &self.header
    }

    #[inline]
    pub fn encoded_size_limit(&self) -> usize {
        self.header.encoded_size_limit()
    }

    #[inline]
    pub fn encode_to_buf(&self, mut buf: impl AsMut<[u8]>) -> Result<usize> {
        let buf = buf.as_mut();
        let size_required = self.encoded_size_limit();
        if unlikely(buf.len() < size_required) {
            return Err(Error::OutputBufferTooSmall { size: buf.len(), required: size_required });
        }
        let (head, tail) = buf.split_at_mut(QOI_HEADER_SIZE); // can't panic
        head.copy_from_slice(&self.header.encode());
        let n_written = qoi_encode_impl_all(BytesMut::new(tail), self.data, self.header.channels)?;
        Ok(QOI_HEADER_SIZE + n_written)
    }

    #[inline]
    pub fn encode_to_vec(&self) -> Result<Vec<u8>> {
        let mut out = vec![0_u8; self.encoded_size_limit()];
        let size = self.encode_to_buf(&mut out)?;
        out.truncate(size);
        Ok(out)
    }

    #[inline]
    pub fn encode_to_stream<W: Write>(&self, writer: &mut W) -> Result<usize> {
        writer.write_all(&self.header.encode())?;
        let n_written =
            qoi_encode_impl_all(GenericWriter::new(writer), self.data, self.header.channels)?;
        Ok(n_written + QOI_HEADER_SIZE)
    }
}
