mod colorspace;
mod consts;
mod decode;
mod encode;
mod error;
mod header;
mod pixel;

pub use crate::colorspace::ColorSpace;
pub use crate::decode::qoi_decode_to_vec;
pub use crate::encode::qoi_encode_to_vec;
pub use crate::error::{Error, Result};

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::path::PathBuf;

    use crate::{consts::QOI_MAGIC, qoi_decode_to_vec, qoi_encode_to_vec};

    fn read_png(rel_path: &str) -> (u32, u32, u8, Vec<u8>) {
        let get_path = || -> Option<PathBuf> {
            Some(PathBuf::from(file!()).parent()?.parent()?.join(rel_path))
        };
        let decoder = png::Decoder::new(File::open(get_path().unwrap()).unwrap());
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let bytes = &buf[..info.buffer_size()];
        (info.width, info.height, info.color_type.samples() as u8, bytes.to_vec())
    }

    #[test]
    fn kodim_01() {
        let (w, h, c, v) = read_png("assets/kodim01.png");
        let q = qoi_encode_to_vec(&v, w, h, c, 0).unwrap();
        std::fs::write("kodim01.qoi", q.as_slice()).unwrap();
    }

    #[test]
    fn wikipedia() {
        let (w, h, c, v) = read_png("assets/en.wikipedia.org.png");
        let q = qoi_encode_to_vec(&v, w, h, c, 0).unwrap();
        std::fs::write("wikipedia.qoi", q.as_slice()).unwrap();
    }

    #[test]
    fn roundtrip_3() {
        let three_raw = include_bytes!("../assets/three.raw").to_vec();
        let (w, h, c) = (572, 354, 3);
        let three_qoi = qoi_encode_to_vec(&three_raw, w, h, c, 0).unwrap();
        let (header, three_rtp) = qoi_decode_to_vec(&three_qoi, c).unwrap();
        assert_eq!(header.magic, QOI_MAGIC);
        assert_eq!(header.width, w);
        assert_eq!(header.height, h);
        assert_eq!(header.channels, c);
        assert_eq!(three_rtp.len(), (w as usize) * (h as usize) * 3);
        assert_eq!(three_raw, three_rtp.as_slice());
    }
}
