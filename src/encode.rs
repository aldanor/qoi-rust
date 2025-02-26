#[cfg(any(feature = "std", feature = "alloc"))]
use alloc::{vec, vec::Vec};
use core::convert::TryFrom;
#[cfg(feature = "std")]
use std::io::Write;

use bytemuck::Pod;

use crate::consts::{QOI_HEADER_SIZE, QOI_OP_INDEX, QOI_OP_RUN, QOI_PADDING, QOI_PADDING_SIZE};
use crate::error::{Error, Result};
use crate::header::Header;
use crate::pixel::{Pixel, SupportedChannels};
use crate::types::{Channels, ColorSpace, RawChannels};
#[cfg(feature = "std")]
use crate::utils::GenericWriter;
use crate::utils::{unlikely, BytesMut, Writer};

#[allow(clippy::cast_possible_truncation, unused_assignments, unused_variables)]
fn encode_impl<W: Writer, const N: usize, const R: usize>(
    mut buf: W, data: &[u8], width: usize, height: usize, stride: usize,
    read_px: impl Fn(&mut Pixel<N>, &[u8]),
) -> Result<usize>
where
    Pixel<N>: SupportedChannels,
    [u8; N]: Pod,
{
    let cap = buf.capacity();

    let mut index = [Pixel::new(); 256];
    let mut px_prev = Pixel::new().with_a(0xff);
    let mut hash_prev = px_prev.hash_index();
    let mut run = 0_u8;
    let mut px = Pixel::<N>::new().with_a(0xff);
    let mut index_allowed = false;

    let n_pixels = width * height;

    let mut i = 0;
    for row in data.chunks(stride).take(height) {
        let pixel_row = &row[..width * R];
        for chunk in pixel_row.chunks_exact(R) {
            read_px(&mut px, chunk);
            if px == px_prev {
                run += 1;
                if run == 62 || unlikely(i == n_pixels - 1) {
                    buf = buf.write_one(QOI_OP_RUN | (run - 1))?;
                    run = 0;
                }
            } else {
                if run != 0 {
                    #[cfg(not(feature = "reference"))]
                    {
                        // credits for the original idea: @zakarumych (had to be fixed though)
                        buf = buf.write_one(if run == 1 && index_allowed {
                            QOI_OP_INDEX | hash_prev
                        } else {
                            QOI_OP_RUN | (run - 1)
                        })?;
                    }
                    #[cfg(feature = "reference")]
                    {
                        buf = buf.write_one(QOI_OP_RUN | (run - 1))?;
                    }
                    run = 0;
                }
                index_allowed = true;
                let px_rgba = px.as_rgba(0xff);
                hash_prev = px_rgba.hash_index();
                let index_px = &mut index[hash_prev as usize];
                if *index_px == px_rgba {
                    buf = buf.write_one(QOI_OP_INDEX | hash_prev)?;
                } else {
                    *index_px = px_rgba;
                    buf = px.encode_into(px_prev, buf)?;
                }
                px_prev = px;
            }
            i += 1;
        }
    }

    assert_eq!(i, n_pixels);
    buf = buf.write_many(&QOI_PADDING)?;
    Ok(cap.saturating_sub(buf.capacity()))
}

/// The maximum number of bytes the encoded image will take.
///
/// Can be used to pre-allocate the buffer to encode the image into.
#[inline]
pub fn encode_max_len(width: u32, height: u32, channels: impl Into<u8>) -> usize {
    let (width, height) = (width as usize, height as usize);
    let n_pixels = width.saturating_mul(height);
    QOI_HEADER_SIZE
        + n_pixels.saturating_mul(channels.into() as usize)
        + n_pixels
        + QOI_PADDING_SIZE
}

/// Encode the image into a pre-allocated buffer.
///
/// Returns the total number of bytes written.
#[inline]
pub fn encode_to_buf(
    buf: impl AsMut<[u8]>, data: impl AsRef<[u8]>, width: u32, height: u32,
) -> Result<usize> {
    Encoder::new(&data, width, height)?.encode_to_buf(buf)
}

/// Encode the image into a newly allocated vector.
#[cfg(any(feature = "alloc", feature = "std"))]
#[inline]
pub fn encode_to_vec(data: impl AsRef<[u8]>, width: u32, height: u32) -> Result<Vec<u8>> {
    Encoder::new(&data, width, height)?.encode_to_vec()
}

/// Encode QOI images into buffers or into streams.
pub struct Encoder<'a> {
    data: &'a [u8],
    stride: usize,
    raw_channels: RawChannels,
    header: Header,
}

impl<'a> Encoder<'a> {
    /// Creates a new encoder from a given array of pixel data and image dimensions.
    /// The data must be in RGB(A) order, without fill borders (extra stride).
    ///
    /// The number of channels will be inferred automatically (the valid values
    /// are 3 or 4). The color space will be set to sRGB by default.
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
        let raw_channels = RawChannels::from(header.channels);
        let stride = width as usize * raw_channels.bytes_per_pixel();
        Ok(Self { data, stride, raw_channels, header })
    }

    /// Creates a new encoder from a given array of pixel data, image
    /// dimensions, stride, and raw format.
    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    pub fn new_raw(
        data: &'a (impl AsRef<[u8]> + ?Sized), width: u32, height: u32, stride: usize,
        raw_channels: RawChannels,
    ) -> Result<Self> {
        let data = data.as_ref();
        let channels = raw_channels.into();
        let header = Header::try_new(width, height, channels, ColorSpace::default())?;

        if stride < width as usize * raw_channels.bytes_per_pixel() {
            return Err(Error::InvalidStride { stride });
        }
        let size = data.len();
        if stride * (height - 1) as usize + width as usize * raw_channels.bytes_per_pixel() < size {
            return Err(Error::InvalidImageLength { size, width, height });
        }

        Ok(Self { data, stride, raw_channels, header })
    }

    /// Returns a new encoder with modified color space.
    ///
    /// Note: the color space doesn't affect encoding or decoding in any way, it's
    /// a purely informative field that's stored in the image header.
    #[inline]
    pub const fn with_colorspace(mut self, colorspace: ColorSpace) -> Self {
        self.header = self.header.with_colorspace(colorspace);
        self
    }

    /// Returns the inferred number of channels.
    #[inline]
    pub const fn channels(&self) -> Channels {
        self.header.channels
    }

    /// Returns the header that will be stored in the encoded image.
    #[inline]
    pub const fn header(&self) -> &Header {
        &self.header
    }

    /// The maximum number of bytes the encoded image will take.
    ///
    /// Can be used to pre-allocate the buffer to encode the image into.
    #[inline]
    pub fn required_buf_len(&self) -> usize {
        self.header.encode_max_len()
    }

    /// Encodes the image to a pre-allocated buffer and returns the number of bytes written.
    ///
    /// The minimum size of the buffer can be found via [`Encoder::required_buf_len`].
    #[inline]
    pub fn encode_to_buf(&self, mut buf: impl AsMut<[u8]>) -> Result<usize> {
        let buf = buf.as_mut();
        let size_required = self.required_buf_len();
        if unlikely(buf.len() < size_required) {
            return Err(Error::OutputBufferTooSmall { size: buf.len(), required: size_required });
        }
        let (head, tail) = buf.split_at_mut(QOI_HEADER_SIZE); // can't panic
        head.copy_from_slice(&self.header.encode());
        let n_written = self.encode_impl_all(BytesMut::new(tail))?;
        Ok(QOI_HEADER_SIZE + n_written)
    }

    /// Encodes the image into a newly allocated vector of bytes and returns it.
    #[cfg(any(feature = "alloc", feature = "std"))]
    #[inline]
    pub fn encode_to_vec(&self) -> Result<Vec<u8>> {
        let mut out = vec![0_u8; self.required_buf_len()];
        let size = self.encode_to_buf(&mut out)?;
        out.truncate(size);
        Ok(out)
    }

    /// Encodes the image directly to a generic writer that implements [`Write`](std::io::Write).
    ///
    /// Note: while it's possible to pass a `&mut [u8]` slice here since it implements `Write`,
    /// it would more effficient to use a specialized method instead: [`Encoder::encode_to_buf`].
    #[cfg(feature = "std")]
    #[inline]
    pub fn encode_to_stream<W: Write>(&self, writer: &mut W) -> Result<usize> {
        writer.write_all(&self.header.encode())?;
        let n_written = self.encode_impl_all(GenericWriter::new(writer))?;
        Ok(n_written + QOI_HEADER_SIZE)
    }

    #[inline]
    fn encode_impl_all<W: Writer>(&self, out: W) -> Result<usize> {
        let width = self.header.width as usize;
        let height = self.header.height as usize;
        let stride = self.stride;
        match self.raw_channels {
            RawChannels::Rgb => {
                encode_impl::<_, 3, 3>(out, self.data, width, height, stride, |px, c| px.read(c))
            }
            RawChannels::Bgr => {
                encode_impl::<_, 3, 3>(out, self.data, width, height, stride, |px, c| {
                    px.update_rgb(c[2], c[1], c[0]);
                })
            }
            RawChannels::Rgba => {
                encode_impl::<_, 4, 4>(out, self.data, width, height, stride, |px, c| px.read(c))
            }
            RawChannels::Argb => {
                encode_impl::<_, 4, 4>(out, self.data, width, height, stride, |px, c| {
                    px.update_rgba(c[1], c[2], c[3], c[0])
                })
            }
            RawChannels::Rgbx => {
                encode_impl::<_, 3, 4>(out, self.data, width, height, stride, |px, c| {
                    px.read(&c[..3])
                })
            }
            RawChannels::Xrgb => {
                encode_impl::<_, 3, 4>(out, self.data, width, height, stride, |px, c| {
                    px.update_rgb(c[1], c[2], c[3])
                })
            }
            RawChannels::Bgra => {
                encode_impl::<_, 4, 4>(out, self.data, width, height, stride, |px, c| {
                    px.update_rgba(c[2], c[1], c[0], c[3])
                })
            }
            RawChannels::Abgr => {
                encode_impl::<_, 4, 4>(out, self.data, width, height, stride, |px, c| {
                    px.update_rgba(c[3], c[2], c[1], c[0])
                })
            }
            RawChannels::Bgrx => {
                encode_impl::<_, 3, 4>(out, self.data, width, height, stride, |px, c| {
                    px.update_rgb(c[2], c[1], c[0])
                })
            }
            RawChannels::Xbgr => {
                encode_impl::<_, 4, 4>(out, self.data, width, height, stride, |px, c| {
                    px.update_rgb(c[3], c[2], c[1])
                })
            }
        }
    }
}
