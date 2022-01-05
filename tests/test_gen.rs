mod common;

use cfg_if::cfg_if;
use rand::{rngs::StdRng, Rng, SeedableRng};

use libqoi::{qoi_decode, qoi_encode};
use qoi_fast::{decode_to_vec, encode_to_vec};

use self::common::hash;

struct GenState<const N: usize> {
    index: [[u8; N]; 64],
    pixels: Vec<u8>,
    prev: [u8; N],
    len: usize,
}

impl<const N: usize> GenState<N> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            index: [[0; N]; 64],
            pixels: Vec::with_capacity(capacity * N),
            prev: Self::zero(),
            len: 0,
        }
    }
    pub fn write(&mut self, px: [u8; N]) {
        self.index[hash(px) as usize] = px;
        for i in 0..N {
            self.pixels.push(px[i]);
        }
        self.prev = px;
        self.len += 1;
    }

    pub fn pick_from_index(&self, rng: &mut impl Rng) -> [u8; N] {
        self.index[rng.gen_range(0_usize..64)]
    }

    pub fn zero() -> [u8; N] {
        let mut px = [0; N];
        if N >= 4 {
            px[3] = 0xff;
        }
        px
    }
}

struct ImageGen {
    p_new: f64,
    p_index: f64,
    p_repeat: f64,
    p_diff: f64,
    p_luma: f64,
}

impl ImageGen {
    pub fn new_random(rng: &mut impl Rng) -> Self {
        let p = [0; 6].map(|_| rng.gen::<f64>());
        let t = p.iter().sum::<f64>();
        Self {
            p_new: p[0] / t,
            p_index: p[1] / t,
            p_repeat: p[2] / t,
            p_diff: p[3] / t,
            p_luma: p[4] / t,
        }
    }

    pub fn generate(&self, rng: &mut impl Rng, channels: usize, min_len: usize) -> Vec<u8> {
        match channels {
            3 => self.generate_const::<_, 3>(rng, min_len),
            4 => self.generate_const::<_, 4>(rng, min_len),
            _ => panic!(),
        }
    }

    fn generate_const<R: Rng, const N: usize>(&self, rng: &mut R, min_len: usize) -> Vec<u8> {
        let mut s = GenState::<N>::with_capacity(min_len);
        let zero = GenState::<N>::zero();

        while s.len < min_len {
            let mut p = rng.gen_range(0.0..1.0);

            if p < self.p_new {
                s.write([0; N].map(|_| rng.gen()));
                continue;
            }
            p -= self.p_new;

            if p < self.p_index {
                let px = s.pick_from_index(rng);
                s.write(px);
                continue;
            }
            p -= self.p_index;

            if p < self.p_repeat {
                let px = s.prev;
                let n_repeat = rng.gen_range(1_usize..=70);
                for _ in 0..n_repeat {
                    s.write(px);
                }
                continue;
            }
            p -= self.p_repeat;

            if p < self.p_diff {
                let mut px = s.prev;
                let d = [0; 3].map(|_| rng.gen_range(0_u8..4).wrapping_sub(2));
                px[0] = px[0].wrapping_add(d[0]);
                px[1] = px[1].wrapping_add(d[0]);
                px[2] = px[2].wrapping_add(d[0]);
                s.write(px);
                continue;
            }
            p -= self.p_diff;

            if p < self.p_luma {
                let mut px = s.prev;
                let vg = rng.gen_range(0_u8..64).wrapping_sub(32);
                let vr = rng.gen_range(0_u8..16).wrapping_sub(8).wrapping_add(vg);
                let vb = rng.gen_range(0_u8..16).wrapping_sub(8).wrapping_add(vg);
                px[0] = px[0].wrapping_add(vr);
                px[1] = px[1].wrapping_add(vg);
                px[2] = px[2].wrapping_add(vb);
                s.write(px);
                continue;
            }

            s.write(zero);
        }

        s.pixels
    }
}

#[test]
fn test_generated() {
    let mut rng = StdRng::seed_from_u64(0);

    let mut n_pixels = 0;
    while n_pixels < 20_000_000 {
        let min_len = rng.gen_range(1..=5000);
        let channels = rng.gen_range(3..=4);
        let gen = ImageGen::new_random(&mut rng);
        let img = gen.generate(&mut rng, channels, min_len);
        let size = img.len() / channels;

        let encoded = encode_to_vec(&img, size as _, 1).unwrap();
        let decoded = decode_to_vec(&encoded).unwrap().1;
        assert_eq!(&img, &decoded, "qoi-fast: roundtrip fail");

        let encoded_c = qoi_encode(&img, size as _, 1, channels as _).unwrap();
        let decoded_c = qoi_decode(encoded_c.as_ref(), channels as _).unwrap().1;
        assert_eq!(&img, decoded_c.as_ref(), "qoi.h: roundtrip fail");

        cfg_if! {
            if #[cfg(feature = "reference")] {
                assert_eq!(&encoded, encoded_c.as_ref(), "qoi-fast[reference] doesn't match qoi.h");
            }
        }

        let encoded_internal_decoded_c = qoi_decode(encoded.as_ref(), channels as _).unwrap().1;
        assert_eq!(encoded_internal_decoded_c.as_ref(), &img, "qoi-fast -> qoi.h: roundtrip fail");

        let encoded_c_decoded_internal = decode_to_vec(encoded_c.as_ref()).unwrap().1;
        assert_eq!(&encoded_c_decoded_internal, &img, "qoi.h -> qoi-fast: roundtrip fail");

        n_pixels += size;
    }
}
