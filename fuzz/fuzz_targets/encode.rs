#![no_main]
use libfuzzer_sys::fuzz_target;

use qoi::{encode_max_len, encode_to_vec};

fuzz_target!(|input: (bool, u8, &[u8])| {
    let (is_4, w_frac, data) = input;
    let channels = if is_4 { 4 } else { 3 };
    let size = data.len();
    let n_pixels = size / channels as usize;
    let (w, h) = if n_pixels == 0 {
        (0, 0)
    } else {
        let w = ((n_pixels * (1 + w_frac as usize)) / 256).max(1);
        let h = n_pixels / w;
        (w, h)
    };
    let out = encode_to_vec(&data[..(w * h * channels as usize)], w as u32, h as u32);
    if w * h != 0 {
        let out = out.unwrap();
        assert!(out.len() <= encode_max_len(w as u32, h as u32, channels));
    } else {
        assert!(out.is_err());
    }
});
