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

#[cfg(target_endian = "big")]
#[test]
fn test_big_endian() {
    // so we can see it in the CI logs
    assert_eq!(u16::to_be_bytes(1), [0, 1]);
}
