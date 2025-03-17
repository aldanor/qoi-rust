use qoi::{
    consts::{QOI_OP_RGB, QOI_OP_RUN},
    decode_to_vec, Channels, ColorSpace, Error, Header, RawChannels, Result,
};

#[test]
fn test_new_decoder() {
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
    let (_, decoded) = decode_to_vec(&qoi_data)?;
    assert_eq!(decoded, vec![0, 0, 0, 255, 0, 0, 0, 255, 10, 20, 30, 255]);
    Ok(())
}

#[cfg(target_endian = "big")]
#[test]
fn test_big_endian() {
    // so we can see it in the CI logs
    assert_eq!(u16::to_be_bytes(1), [0, 1]);
}

#[test]
fn test_new_encoder() {
    let arr3 = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]; // 2 * 2 * 3
    let arr4 = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]; // 2 * 2 * 4

    let enc = qoi::Encoder::new(&arr3, 2, 2).unwrap();
    assert_eq!(enc.channels(), Channels::Rgb);

    let enc = qoi::Encoder::new(&arr4, 2, 2).unwrap();
    assert_eq!(enc.channels(), Channels::Rgba);

    assert!(matches!(
        qoi::Encoder::new(&arr3, 3, 3),
        Err(Error::InvalidImageLength { size: 12, width: 3, height: 3 })
    ));

    assert!(matches!(qoi::Encoder::new(&arr3, 1, 1), Err(Error::InvalidChannels { channels: 12 })));

    let enc = qoi::Encoder::new_raw(&arr3, 2, 2, 2 * 3, RawChannels::Bgr).unwrap();
    assert_eq!(enc.channels(), Channels::Rgb);
    let qoi = enc.encode_to_vec().unwrap();
    let (_header, res) = decode_to_vec(qoi).unwrap();
    assert_eq!(res, [2, 1, 0, 5, 4, 3, 8, 7, 6, 11, 10, 9]);

    let enc = qoi::Encoder::new_raw(&arr4, 2, 2, 2 * 4, RawChannels::Rgba).unwrap();
    assert_eq!(enc.channels(), Channels::Rgba);
    let qoi = enc.encode_to_vec().unwrap();
    let (_header, res) = decode_to_vec(qoi).unwrap();
    assert_eq!(res, arr4);

    let enc = qoi::Encoder::new_raw(&arr4, 2, 2, 2 * 4, RawChannels::Bgra).unwrap();
    assert_eq!(enc.channels(), Channels::Rgba);
    let qoi = enc.encode_to_vec().unwrap();
    let (_header, res) = decode_to_vec(qoi).unwrap();
    assert_eq!(res, [2, 1, 0, 3, 6, 5, 4, 7, 10, 9, 8, 11, 14, 13, 12, 15]);

    let enc = qoi::Encoder::new_raw(&arr4, 2, 2, 2 * 4, RawChannels::Rgbx).unwrap();
    assert_eq!(enc.channels(), Channels::Rgb);
    let qoi = enc.encode_to_vec().unwrap();
    let (_header, res) = decode_to_vec(qoi).unwrap();
    assert_eq!(res, [0, 1, 2, 4, 5, 6, 8, 9, 10, 12, 13, 14]);

    let enc = qoi::Encoder::new_raw(&arr4, 2, 2, 2 * 4, RawChannels::Xrgb).unwrap();
    assert_eq!(enc.channels(), Channels::Rgb);
    let qoi = enc.encode_to_vec().unwrap();
    let (_header, res) = decode_to_vec(qoi).unwrap();
    assert_eq!(res, [1, 2, 3, 5, 6, 7, 9, 10, 11, 13, 14, 15]);

    let enc = qoi::Encoder::new_raw(&arr4, 2, 2, 2 * 4, RawChannels::Bgra).unwrap();
    assert_eq!(enc.channels(), Channels::Rgba);
    let qoi = enc.encode_to_vec().unwrap();
    let (_header, res) = decode_to_vec(qoi).unwrap();
    assert_eq!(res, [2, 1, 0, 3, 6, 5, 4, 7, 10, 9, 8, 11, 14, 13, 12, 15]);

    let enc = qoi::Encoder::new_raw(&arr4, 2, 2, 2 * 4, RawChannels::Abgr).unwrap();
    assert_eq!(enc.channels(), Channels::Rgba);
    let qoi = enc.encode_to_vec().unwrap();
    let (_header, res) = decode_to_vec(qoi).unwrap();
    assert_eq!(res, [3, 2, 1, 0, 7, 6, 5, 4, 11, 10, 9, 8, 15, 14, 13, 12]);

    let enc = qoi::Encoder::new_raw(&arr4, 2, 2, 2 * 4, RawChannels::Bgrx).unwrap();
    assert_eq!(enc.channels(), Channels::Rgb);
    let qoi = enc.encode_to_vec().unwrap();
    let (_header, res) = decode_to_vec(qoi).unwrap();
    assert_eq!(res, [2, 1, 0, 6, 5, 4, 10, 9, 8, 14, 13, 12]);

    let enc = qoi::Encoder::new_raw(&arr4, 2, 2, 2 * 4, RawChannels::Xbgr).unwrap();
    assert_eq!(enc.channels(), Channels::Rgb);
    let qoi = enc.encode_to_vec().unwrap();
    let (_header, res) = decode_to_vec(qoi).unwrap();
    assert_eq!(res, [3, 2, 1, 7, 6, 5, 11, 10, 9, 15, 14, 13]);
}
