#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: (u16, u16, bool, &[u8])| {
    let (w, h, is_4, data) = input;
    let (w, h) = (w % 300, h % 300);
    let channels = if is_4 { 4 } else { 3 };

    let mut vec = vec![
        b'q',
        b'o',
        b'i',
        b'f',
        0,
        0,
        (w >> 8) as u8,
        (w & 0xff) as u8,
        0,
        0,
        (h >> 8) as u8,
        (h & 0xff) as u8,
        channels,
        0,
    ];
    vec.extend(&*data);
    vec.extend(&[0, 0, 0, 0]);

    let out = qoi_fast::qoi_decode_to_vec(&vec, channels);
    if let Ok((header, out)) = out {
        assert_eq!(header.magic, qoi_fast::consts::QOI_MAGIC);
        assert_eq!(header.width, w as u32);
        assert_eq!(header.height, h as u32);
        assert_eq!(header.channels, channels);
        assert_eq!(header.colorspace.to_u8(), 0);
        assert_eq!(out.len(), w as usize * h as usize * channels as usize);
    }
});
