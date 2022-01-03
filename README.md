# qoi-fast

VERY fast encoder/decoder for [QOI image format](https://qoiformat.org/), implemented in pure Rust.

[![Build](https://github.com/aldanor/qoi-fast/workflows/CI/badge.svg)](https://github.com/aldanor/qoi-fast/actions?query=branch%3Amaster)
[![Latest Version](https://img.shields.io/crates/v/qoi-fast.svg)](https://crates.io/crates/qoi-fast)
[![Documentation](https://img.shields.io/docsrs/qoi-fast)](https://docs.rs/qoi-fast)
[![Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance)

Quick summary:

- One of the [fastest](https://github.com/aldanor/qoi-fast#benchmarks)
  QOI encoders/decoders out there.
- Compliant with the latest QOI [format specification](https://qoiformat.org/qoi-specification.pdf).
- Zero unsafe code.
- Supports decoding from / encoding to `std::io` streams directly.
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

qoi-c            4409.34      5531.82        282.3        225.0
qoi-fast         3105.60      4726.19        400.8        263.3
```

### License

This project is dual-licensed under MIT and Apache 2.0.
