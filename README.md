# [qoi-fast](https://crates.io/crates/qoi-fast)

[![Build](https://github.com/aldanor/qoi-fast/workflows/CI/badge.svg)](https://github.com/aldanor/qoi-fast/actions?query=branch%3Amaster)
[![Latest Version](https://img.shields.io/crates/v/qoi-fast.svg)](https://crates.io/crates/qoi-fast)
[![Documentation](https://img.shields.io/docsrs/qoi-fast)](https://docs.rs/qoi-fast)
[![Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance)

Fast encoder/decoder for [QOI image format](https://qoiformat.org/), implemented in pure and safe Rust.

- One of the [fastest](#benchmarks) QOI encoders/decoders out there.
- Compliant with the [latest](https://qoiformat.org/qoi-specification.pdf) QOI format specification.
- Zero unsafe code.
- Supports decoding from / encoding to `std::io` streams directly.
- `no_std` support.
- Roundtrip-tested vs the reference C implementation; fuzz-tested.

### Examples

```rust
use qoi_fast::{encode_to_vec, decode_to_vec};

let encoded = encode_to_vec(&pixels, width, height)?;
let (header, decoded) = decode_to_vec(&encoded)?;

assert_eq!(header.width, width);
assert_eq!(header.height, height);
assert_eq!(decoded, pixels);
```

### Benchmarks

Comparison to the reference C implementation
(as of [00e34217](https://github.com/phoboslab/qoi/commit/00e34217)),
benchmarks timings collected on Apple M1 (1782 images, 1187 MB total):

```
codec          decode:ms    encode:ms  decode:mp/s  encode:mp/s

qoi-c            4389.75      5524.18        283.5        225.3
qoi-fast         3026.68      4304.26        411.2        289.2
```

Benchmarks have also been run for all of the other Rust implementations
of QOI for comparison purposes and, at the time of writing this document,
this library proved to be the fastest one by a noticeable margin.

### `no_std`

This crate supports `no_std` mode. By default, std is enabled via the `std`
feature. You can deactivate the `default-features` to target core instead.
In that case anything related to `std::io`, `std::error::Error` and heap
allocations is disabled. There is an additional `alloc` feature that can
be activated to bring back the support for heap allocations.

### License

This project is dual-licensed under MIT and Apache 2.0.
