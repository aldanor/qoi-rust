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
#![cfg_attr(not(any(feature = "std", test)), no_std)]
#[cfg(all(feature = "alloc", not(any(feature = "std", test))))]
extern crate alloc;
#[cfg(any(feature = "std", test))]
extern crate std as alloc;

mod decode;
mod encode;
mod error;
mod header;
mod pixel;
mod types;
mod utils;

#[doc(hidden)]
pub mod consts;

#[cfg(any(feature = "alloc", feature = "std"))]
pub use crate::decode::qoi_decode_to_vec;
#[cfg(feature = "std")]
pub use crate::decode::QoiStreamDecoder;
pub use crate::decode::{qoi_decode_header, qoi_decode_to_buf, QoiDecoder};

#[cfg(any(feature = "alloc", feature = "std"))]
pub use crate::encode::qoi_encode_to_vec;
pub use crate::encode::{encoded_size_limit, qoi_encode_to_buf, QoiEncoder};
pub use crate::error::{Error, Result};
pub use crate::header::Header;
pub use crate::types::{Channels, ColorSpace};
