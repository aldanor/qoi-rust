pub const QOI_INDEX: u8 = 0x00;
pub const QOI_RUN_8: u8 = 0x40;
pub const QOI_RUN_16: u8 = 0x60;
pub const QOI_DIFF_8: u8 = 0x80;
pub const QOI_DIFF_16: u8 = 0xc0;
pub const QOI_DIFF_24: u8 = 0xe0;
pub const QOI_COLOR: u8 = 0xf0;

pub const QOI_MASK_2: u8 = 0xc0;
pub const QOI_MASK_3: u8 = 0xe0;
pub const QOI_MASK_4: u8 = 0xf0;

pub const QOI_HEADER_SIZE: usize = 14;
pub const QOI_PADDING: usize = 4;

pub const QOI_MAGIC: [u8; 4] = [b'q', b'o', b'i', b'f'];
