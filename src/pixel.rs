#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct Pixel<const N: usize>([u8; N]);

impl<const N: usize> Pixel<N> {
    #[inline]
    pub const fn new() -> Self {
        Self([0; N])
    }

    #[inline]
    pub fn read(&mut self, s: &[u8]) {
        let mut i = 0;
        while i < N {
            self.0[i] = s[i];
            i += 1;
        }
    }

    #[inline]
    pub const fn as_rgba(self, with_a: u8) -> Pixel<4> {
        let mut i = 0;
        let mut out = Pixel::new();
        while i < N {
            out.0[i] = self.0[i];
            i += 1;
        }
        if N < 4 {
            out.0[3] = with_a;
        }
        out
    }

    #[inline]
    pub const fn from_rgb(px: Pixel<3>, with_a: u8) -> Self {
        let mut i = 0;
        let mut out = Self::new();
        while i < 3 {
            out.0[i] = px.0[i];
            i += 1;
        }
        out.with_a(with_a)
    }

    #[inline]
    pub const fn from_array<const M: usize>(arr: [u8; M]) -> Self {
        let mut i = 0;
        let mut out = Self::new();
        while i < N && i < M {
            out.0[i] = arr[i];
            i += 1;
        }
        out
    }

    #[inline]
    pub const fn r(self) -> u8 {
        self.0[0]
    }

    #[inline]
    pub const fn g(self) -> u8 {
        self.0[1]
    }

    #[inline]
    pub const fn b(self) -> u8 {
        self.0[2]
    }

    #[inline]
    pub const fn with_a(mut self, value: u8) -> Self {
        if N >= 4 {
            self.0[3] = value;
        }
        self
    }

    #[inline]
    pub const fn a_or(self, value: u8) -> u8 {
        if N < 4 {
            value
        } else {
            self.0[3]
        }
    }

    #[inline]
    pub const fn hash_index(self) -> u8 {
        let r = self.r().wrapping_mul(3);
        let g = self.g().wrapping_mul(5);
        let b = self.b().wrapping_mul(7);
        let a = self.a_or(0xff).wrapping_mul(11);
        r.wrapping_add(g).wrapping_add(b).wrapping_add(a) % 64
    }

    #[inline]
    pub fn rgb_add(&mut self, r: u8, g: u8, b: u8) {
        self.0[0] = self.0[0].wrapping_add(r);
        self.0[1] = self.0[1].wrapping_add(g);
        self.0[2] = self.0[2].wrapping_add(b);
    }
}

impl<const N: usize> From<Pixel<N>> for [u8; N] {
    #[inline(always)]
    fn from(px: Pixel<N>) -> Self {
        px.0
    }
}

pub trait SupportedChannels {}

impl SupportedChannels for Pixel<3> {}
impl SupportedChannels for Pixel<4> {}
