#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(
    clippy::inline_always,
    clippy::similar_names,
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::module_name_repetitions,
    clippy::cargo_common_metadata
)]

mod colorspace;
mod decode;
mod encode;
mod error;
mod header;
mod pixel;
mod utils;

#[doc(hidden)]
pub mod consts;

pub use crate::colorspace::ColorSpace;
pub use crate::decode::{
    qoi_decode_header, qoi_decode_to_buf, qoi_decode_to_vec, QoiDecoder, QoiStreamDecoder,
};
pub use crate::encode::{encode_size_required, qoi_encode_to_buf, qoi_encode_to_vec};
pub use crate::error::{Error, Result};
pub use crate::header::Header;
