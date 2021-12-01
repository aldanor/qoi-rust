use std::fmt::{self, Debug};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ColorSpace {
    pub r_linear: bool,
    pub g_linear: bool,
    pub b_linear: bool,
    pub a_linear: bool,
}

impl ColorSpace {
    pub const SRGB: Self = Self::new(false, false, false, false);
    pub const LINEAR: Self = Self::new(true, true, true, true);
    pub const SRGB_LINEAR_ALPHA: Self = Self::new(false, false, false, true);

    pub const fn new(r_linear: bool, g_linear: bool, b_linear: bool, a_linear: bool) -> Self {
        Self { r_linear, g_linear, b_linear, a_linear }
    }

    pub const fn is_srgb(self) -> bool {
        !self.r_linear && !self.g_linear && !self.b_linear && !self.a_linear
    }

    pub const fn is_linear(self) -> bool {
        self.r_linear && self.g_linear && self.b_linear && self.a_linear
    }

    pub const fn is_srgb_linear_alpha(self) -> bool {
        !self.r_linear && !self.g_linear && !self.b_linear && self.a_linear
    }

    pub const fn to_u8(self) -> u8 {
        (self.r_linear as u8) << 3
            | (self.g_linear as u8) << 2
            | (self.b_linear as u8) << 1
            | (self.a_linear as u8)
    }

    pub const fn from_u8(bits: u8) -> Self {
        Self::new(bits & 0x08 != 0, bits & 0x04 != 0, bits & 0x02 != 0, bits & 0x01 != 0)
    }
}

impl Default for ColorSpace {
    fn default() -> Self {
        Self::SRGB
    }
}

impl From<u8> for ColorSpace {
    fn from(value: u8) -> Self {
        Self::from_u8(value)
    }
}

impl From<ColorSpace> for u8 {
    fn from(value: ColorSpace) -> Self {
        value.to_u8()
    }
}

impl Debug for ColorSpace {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ColorSpace({}{}{}{})",
            self.r_linear as u8, self.g_linear as u8, self.b_linear as u8, self.a_linear as u8
        )
    }
}
