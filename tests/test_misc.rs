use qoi::{
    consts::{QOI_OP_INDEX, QOI_OP_RGB, QOI_OP_RUN},
    decode_to_vec, Channels, ColorSpace, Header, Result,
};

#[test]
fn test_new_encoder() {
    // this used to fail due to `Bytes` not being `pub`
    let arr = [0u8];
    let _ = qoi::Decoder::new(&arr[..]);
}

#[test]
fn test_start_with_qoi_op_run() -> Result<()> {
    let header = Header::try_new(3, 1, Channels::Rgba, ColorSpace::Linear)?;
    let mut qoi_data: Vec<_> = header.encode().into_iter().collect();
    qoi_data.extend([QOI_OP_RUN | 1, QOI_OP_RGB, 10, 20, 30]);
    qoi_data.extend([0; 7]);
    qoi_data.push(1);
    let expected = vec![0, 0, 0, 255, 0, 0, 0, 255, 10, 20, 30, 255];

    assert_eq!(decode_to_vec(&qoi_data)?.1, expected);

    #[cfg(feature = "std")]
    {
        let stream = std::io::Cursor::new(&qoi_data);
        let mut decoder = qoi::Decoder::from_stream(stream)?;
        assert_eq!(decoder.decode_to_vec()?, expected);
    }

    Ok(())
}

#[test]
fn test_start_with_qoi_op_run_and_use_index() -> Result<()> {
    let header = Header::try_new(4, 1, Channels::Rgba, ColorSpace::Linear)?;
    let mut qoi_data: Vec<_> = header.encode().into_iter().collect();
    qoi_data.extend([QOI_OP_RUN | 1, QOI_OP_RGB, 10, 20, 30, QOI_OP_INDEX | 53]);
    qoi_data.extend([0; 7]);
    qoi_data.push(1);
    let expected = vec![0, 0, 0, 255, 0, 0, 0, 255, 10, 20, 30, 255, 0, 0, 0, 255];

    assert_eq!(decode_to_vec(&qoi_data)?.1, expected);

    #[cfg(feature = "std")]
    {
        let stream = std::io::Cursor::new(&qoi_data);
        let mut decoder = qoi::Decoder::from_stream(stream)?;
        assert_eq!(decoder.decode_to_vec()?, expected);
    }

    Ok(())
}

#[cfg(feature = "std")]
mod std_tests {
    use qoi::{Channels, ColorSpace, Decoder, Header, Result};

    const ONE_PIXEL_QOI_IMAGE: [u8; 23] = [
        0x71, 0x6f, 0x69, 0x66, // magic
        0x00, 0x00, 0x00, 0x01, // width
        0x00, 0x00, 0x00, 0x01, // height
        0x04, // number of channels
        0x00, // colorspace
        0x55, // QOI_OP_DIFF
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, // padding
    ];

    const ONE_PIXEL_QOI_HEADER: Header =
        Header { width: 1, height: 1, channels: Channels::Rgba, colorspace: ColorSpace::Srgb };

    #[test]
    fn test_decode_stream_to_exact_sized_buffer() -> Result<()> {
        let mut decoder = Decoder::from_stream(&ONE_PIXEL_QOI_IMAGE[..])?;
        assert_eq!(decoder.header(), &ONE_PIXEL_QOI_HEADER);

        let mut out = vec![0u8; decoder.required_buf_len()];
        let n_written = decoder.decode_to_buf(&mut out)?;
        assert_eq!(n_written, 4);
        Ok(())
    }

    #[test]
    fn test_decode_stream_to_larger_buffer() -> Result<()> {
        let mut decoder = Decoder::from_stream(&ONE_PIXEL_QOI_IMAGE[..])?;
        assert_eq!(decoder.header(), &ONE_PIXEL_QOI_HEADER);

        let mut out = vec![0u8; decoder.required_buf_len() + 16];
        let n_written = decoder.decode_to_buf(&mut out)?;
        assert_eq!(n_written, 4);
        assert_eq!(&out[4..], &[0_u8; 16]);
        Ok(())
    }
}

#[cfg(target_endian = "big")]
#[test]
fn test_big_endian() {
    // so we can see it in the CI logs
    assert_eq!(u16::to_be_bytes(1), [0, 1]);
}
