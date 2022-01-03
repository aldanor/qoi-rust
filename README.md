# [qoi-fast](https://crates.io/crates/qoi-fast)

[![Build](https://github.com/aldanor/qoi-fast/workflows/CI/badge.svg)](https://github.com/aldanor/qoi-fast/actions?query=branch%3Amaster)
[![Latest Version](https://img.shields.io/crates/v/qoi-fast.svg)](https://crates.io/crates/qoi-fast)
[![Documentation](https://img.shields.io/docsrs/qoi-fast)](https://docs.rs/qoi-fast)
[![Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance)

Fast encoder/decoder for [QOI image format](https://qoiformat.org/), implemented in pure and safe Rust.

Quick summary:

- One of the [fastest](https://github.com/aldanor/qoi-fast#benchmarks)
  QOI encoders/decoders out there.
- Compliant with the [latest](https://qoiformat.org/qoi-specification.pdf) QOI format specification.
- Zero unsafe code.
- Supports decoding from / encoding to `std::io` streams directly.
- `no_std` support.
- Roundtrip-tested vs the reference C implementation; fuzz-tested.

### Examples

```rust
todo!();
```

### Benchmarks

Comparison to the reference C implementation
(as of [00e34217](https://github.com/phoboslab/qoi/commit/00e34217)),
benchmarks timings collected on Apple M1 (1782 images, 1187 MB total):

```
codec          decode:ms    encode:ms  decode:mp/s  encode:mp/s

qoi-c            4406.63      5515.80        282.4        225.6
qoi-fast         3071.49      4545.08        405.2        273.8
```

### `no_std`

This crate supports `no_std` mode. By default, std is enabled via the `std`
feature. You can deactivate the `default-features` to target core instead.
In that case anything related to `std::io`, `std::error::Error` and heap
allocations is disabled. There is an additional `alloc` feature that can
be activated to bring back the support for heap allocations.

### License

This project is dual-licensed under MIT and Apache 2.0.
