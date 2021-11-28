//! To run benchmarks, also pass RUSTFLAGS="--cfg bench" until cargo does this automatically.

use std::fs::File;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use qoi_fast::*;

pub fn criterion_benchmark(c: &mut Criterion) {
    // let three_raw = include_bytes!("../assets/three.raw");
    // let four_raw = include_bytes!("../assets/four.raw");

    // let three_qoi = qoi_encode::<3>(three_raw, 572, 354).unwrap();
    // c.bench_function("encode 3", |b| {
    //     b.iter(|| black_box(qoi_encode::<3>(three_raw, 572, 354).unwrap()))
    // });
    // c.bench_function("decode 3", |b| {
    //     b.iter(|| black_box(qoi_decode::<3>(&three_qoi).unwrap()))
    // });

    // c.bench_function("encode 4", |b| {
    //     b.iter(|| black_box(qoi_encode::<4>(four_raw, 572, 354).unwrap()))
    // });

    let decoder = png::Decoder::new(
        File::open("/Users/ivansmirnov/projects/rust/qoi-other/images/kodak/kodim11.png").unwrap(),
    );
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!(info.color_type.samples(), 3);
    let png_bytes = &buf[..info.buffer_size()];
    let qoi_bytes = qoi_encode_to_vec(
        png_bytes,
        info.width as _,
        info.height as _,
        info.color_type.samples() as _,
    )
    .unwrap();

    c.bench_function("kodim11.png (encode-3)", |b| {
        b.iter(|| {
            black_box(qoi_encode_to_vec(
                png_bytes,
                info.width as _,
                info.height as _,
                info.color_type.samples() as _,
            ))
            .unwrap()
        })
    });
    c.bench_function("kodim11.png (decode-3)", |b| {
        b.iter(|| black_box(qoi_decode_to_vec(&qoi_bytes, 3)).unwrap())
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
