#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct Pixel<const N: usize>([u8; N]);

impl<const N: usize> Pixel<N> {
    #[inline]
    pub const fn new() -> Self {
        Self([0; N])
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
        (self.r() ^ self.g() ^ self.b() ^ self.a_or(0xff)) % 64
    }

    #[inline]
    pub fn rgb_add(&mut self, r: u8, g: u8, b: u8) {
        self.0[0] = self.0[0].wrapping_add(r);
        self.0[1] = self.0[1].wrapping_add(g);
        self.0[2] = self.0[2].wrapping_add(b);
    }

    #[inline]
    pub fn rgba_add(&mut self, r: u8, g: u8, b: u8, a: u8) {
        self.rgb_add(r, g, b);
        if N >= 4 {
            self.0[3] = self.0[3].wrapping_add(a);
        }
    }

    #[inline]
    pub fn set_r(&mut self, value: u8) {
        self.0[0] = value;
    }

    #[inline]
    pub fn set_g(&mut self, value: u8) {
        self.0[1] = value;
    }

    #[inline]
    pub fn set_b(&mut self, value: u8) {
        self.0[2] = value;
    }

    #[inline]
    pub fn set_a(&mut self, value: u8) {
        if N >= 4 {
            self.0[3] = value;
        }
    }
}

pub trait SupportedChannels {}

impl SupportedChannels for Pixel<3> {}
impl SupportedChannels for Pixel<4> {}
