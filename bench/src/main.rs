use std::cmp::Ordering;
use std::fs::{self, File};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{bail, ensure, Context, Result};
use bytemuck::cast_slice;
use c_vec::CVec;
use qoi::{Decoder, Encoder};
use structopt::StructOpt;
use walkdir::{DirEntry, WalkDir};

fn black_box<T>(dummy: T) -> T {
    unsafe {
        let ret = core::ptr::read_volatile(&dummy);
        core::mem::forget(dummy);
        ret
    }
}

fn timeit<T>(func: impl Fn() -> T) -> (T, Duration) {
    let t0 = Instant::now();
    let out = func();
    let t1 = Instant::now();
    (black_box(out), t1 - t0)
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

fn find_pngs(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let is_png_file = |path: &PathBuf| {
        path.is_file()
            && path.extension().unwrap_or_default().to_string_lossy().to_ascii_lowercase() == "png"
    };

    let mut out = vec![];
    for path in paths {
        if is_png_file(path) {
            out.push(path.clone());
        } else if path.is_dir() {
            out.extend(
                WalkDir::new(path)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(Result::ok)
                    .map(DirEntry::into_path)
                    .filter(is_png_file),
            )
        } else {
            bail!("path doesn't exist: {}", path.to_string_lossy());
        }
    }
    out.sort_unstable();
    Ok(out)
}

fn grayscale_to_rgb(buf: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(buf.len() * 3);
    for &px in buf {
        for _ in 0..3 {
            out.push(px);
        }
    }
    out
}

fn grayscale_alpha_to_rgba(buf: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(buf.len() * 4);
    for &px in cast_slice::<_, [u8; 2]>(buf) {
        for _ in 0..3 {
            out.push(px[0]);
        }
        out.push(px[1])
    }
    out
}

#[derive(Clone)]
struct Image {
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub data: Vec<u8>,
}

impl Image {
    fn read_png(filename: &Path) -> Result<Self> {
        let mut decoder = png::Decoder::new(File::open(filename)?);
        let transformations = png::Transformations::normalize_to_color8();
        decoder.set_transformations(transformations);
        let mut reader = decoder.read_info()?;
        let mut whole_buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut whole_buf)?;
        let buf = &whole_buf[..info.buffer_size()];
        ensure!(info.bit_depth == png::BitDepth::Eight, "invalid bit depth: {:?}", info.bit_depth);
        let (channels, data) = match info.color_type {
            png::ColorType::Grayscale => {
                // png crate doesn't support GRAY_TO_RGB transformation yet
                (3, grayscale_to_rgb(buf))
            }
            png::ColorType::GrayscaleAlpha => {
                // same as above, but with alpha channel
                (4, grayscale_alpha_to_rgba(buf))
            }
            color_type => {
                let channels = color_type.samples();
                ensure!(channels == 3 || channels == 4, "invalid channels: {}", channels);
                (channels as u8, buf[..info.buffer_size()].to_vec())
            }
        };
        Ok(Self { width: info.width, height: info.height, channels, data })
    }

    pub const fn n_pixels(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    pub const fn n_bytes(&self) -> usize {
        self.n_pixels() * (self.channels as usize)
    }
}

trait Codec {
    type Output: AsRef<[u8]>;

    fn name(&self) -> &'static str;
    fn encode(&self, img: &Image) -> Result<Self::Output>;
    fn decode(&self, data: &[u8], img: &Image) -> Result<Self::Output>;
}

struct CodecQoiRust {
    pub stream: bool,
}

impl Codec for CodecQoiRust {
    type Output = Vec<u8>;

    fn name(&self) -> &'static str {
        if self.stream {
            "qoi-rust[stream]"
        } else {
            "qoi-rust"
        }
    }

    fn encode(&self, img: &Image) -> Result<Vec<u8>> {
        if self.stream {
            let mut stream = Vec::new();
            let encoder = Encoder::new(&img.data, img.width, img.height)?;
            encoder.encode_to_stream(&mut stream)?;
            Ok(stream)
        } else {
            Ok(qoi::encode_to_vec(&img.data, img.width, img.height)?)
        }
    }

    fn decode(&self, data: &[u8], _img: &Image) -> Result<Vec<u8>> {
        if self.stream {
            let stream = Cursor::new(data);
            let mut decoder = Decoder::from_stream(stream)?;
            Ok(decoder.decode_to_vec()?)
        } else {
            Ok(qoi::decode_to_vec(data)?.1)
        }
    }
}

struct CodecQoiC;

impl Codec for CodecQoiC {
    type Output = CVec<u8>;

    fn name(&self) -> &'static str {
        "qoi.h"
    }

    fn encode(&self, img: &Image) -> Result<CVec<u8>> {
        libqoi::qoi_encode(&img.data, img.width, img.height, img.channels)
    }

    fn decode(&self, data: &[u8], img: &Image) -> Result<CVec<u8>> {
        Ok(libqoi::qoi_decode(data, img.channels)?.1)
    }
}

#[derive(Clone)]
struct BenchResult {
    pub codec: String,
    pub decode_sec: Vec<f64>,
    pub encode_sec: Vec<f64>,
}

impl BenchResult {
    pub fn new(codec: impl AsRef<str>, mut decode_sec: Vec<f64>, mut encode_sec: Vec<f64>) -> Self {
        decode_sec.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        encode_sec.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        let codec = codec.as_ref().into();
        Self { codec, decode_sec, encode_sec }
    }

    pub fn average_decode_sec(&self, use_median: bool) -> f64 {
        if use_median {
            self.decode_sec[self.decode_sec.len() / 2]
        } else {
            mean(&self.decode_sec)
        }
    }

    pub fn average_encode_sec(&self, use_median: bool) -> f64 {
        if use_median {
            self.encode_sec[self.encode_sec.len() / 2]
        } else {
            mean(&self.encode_sec)
        }
    }
}

#[derive(Clone)]
struct ImageBench {
    results: Vec<BenchResult>,
    n_pixels: usize,
    n_bytes: usize,
}

impl ImageBench {
    pub fn new(img: &Image) -> Self {
        Self { results: vec![], n_pixels: img.n_pixels(), n_bytes: img.n_bytes() }
    }

    pub fn run<C: Codec>(&mut self, codec: &C, img: &Image, sec_allowed: f64) -> Result<()> {
        let (encoded, t_encode) = timeit(|| codec.encode(img));
        let encoded = encoded?;
        let (decoded, t_decode) = timeit(|| codec.decode(encoded.as_ref(), img));
        let decoded = decoded?;
        let roundtrip = decoded.as_ref() == img.data.as_slice();
        if codec.name() == "qoi-rust" {
            assert!(roundtrip, "{}: decoded data doesn't roundtrip", codec.name());
        } else {
            ensure!(roundtrip, "{}: decoded data doesn't roundtrip", codec.name());
        }

        let n_encode = (sec_allowed / 2. / t_encode.as_secs_f64()).max(2.).ceil() as usize;
        let mut encode_tm = Vec::with_capacity(n_encode);
        for _ in 0..n_encode {
            encode_tm.push(timeit(|| codec.encode(img)).1);
        }
        let encode_sec = encode_tm.iter().map(Duration::as_secs_f64).collect();

        let n_decode = (sec_allowed / 2. / t_decode.as_secs_f64()).max(2.).ceil() as usize;
        let mut decode_tm = Vec::with_capacity(n_decode);
        for _ in 0..n_decode {
            decode_tm.push(timeit(|| codec.decode(encoded.as_ref(), img)).1);
        }
        let decode_sec = decode_tm.iter().map(Duration::as_secs_f64).collect();

        self.results.push(BenchResult::new(codec.name(), decode_sec, encode_sec));
        Ok(())
    }

    pub fn report(&self, use_median: bool) {
        let (w_name, w_col) = (9, 13);
        print!("{:<w$}", "", w = w_name);
        print!("{:>w$}", "decode:ms", w = w_col);
        print!("{:>w$}", "encode:ms", w = w_col);
        print!("{:>w$}", "decode:Mp/s", w = w_col);
        print!("{:>w$}", "encode:Mp/s", w = w_col);
        print!("{:>w$}", "decode:MB/s", w = w_col);
        print!("{:>w$}", "encode:MB/s", w = w_col);
        println!();
        for r in &self.results {
            let decode_sec = r.average_decode_sec(use_median);
            let encode_sec = r.average_encode_sec(use_median);
            let mpixels = self.n_pixels as f64 / 1e6;
            let (decode_mpps, encode_mpps) = (mpixels / decode_sec, mpixels / encode_sec);
            let mbytes = self.n_bytes as f64 / 1024. / 1024.;
            let (decode_mbps, encode_mbps) = (mbytes / decode_sec, mbytes / encode_sec);

            print!("{:<w$}", r.codec, w = w_name);
            print!("{:>w$.2}", decode_sec * 1e3, w = w_col);
            print!("{:>w$.2}", encode_sec * 1e3, w = w_col);
            print!("{:>w$.1}", decode_mpps, w = w_col);
            print!("{:>w$.1}", encode_mpps, w = w_col);
            print!("{:>w$.1}", decode_mbps, w = w_col);
            print!("{:>w$.1}", encode_mbps, w = w_col);
            println!();
        }
    }
}

#[derive(Default)]
struct BenchTotals {
    results: Vec<ImageBench>,
}

impl BenchTotals {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, b: &ImageBench) {
        self.results.push(b.clone())
    }

    pub fn report(&self, use_median: bool, fancy: bool) {
        if self.results.is_empty() {
            return;
        }
        let codec_names: Vec<_> = self.results[0].results.iter().map(|r| r.codec.clone()).collect();
        let n_codecs = codec_names.len();
        let (mut total_decode_sec, mut total_encode_sec, mut n_pixels_total, mut n_bytes_total) =
            (vec![0.; n_codecs], vec![0.; n_codecs], 0, 0);
        for r in &self.results {
            n_pixels_total += r.n_pixels;
            n_bytes_total += r.n_bytes;
            for i in 0..n_codecs {
                // sum of medians is not the median of sums, but w/e, good enough here
                total_decode_sec[i] += r.results[i].average_decode_sec(use_median);
                total_encode_sec[i] += r.results[i].average_encode_sec(use_median);
            }
        }
        let mpixels = n_pixels_total as f64 / 1e6;
        let mbytes = n_bytes_total as f64 / 1024. / 1024.;

        println!("---");
        println!(
            "Overall results: ({} images, {:.2} MB raw, {:.2} MP):",
            self.results.len(),
            mbytes,
            mpixels
        );
        if fancy {
            let (w_header, w_col) = (14, 12);
            let n = n_codecs;
            let print_sep = |s| print!("{}{:->w$}", s, "", w = w_header + n * w_col);
            print_sep("");
            print!("\n{:<w$}", "", w = w_header);
            (0..n).for_each(|i| print!("{:>w$}", codec_names[i], w = w_col));
            print_sep("\n");
            // print!("\n{:<w$}", "         ms", w = w_header);
            // (0..n).for_each(|i| print!("{:>w$.2}", total_decode_sec[i] * 1e3, w = w_col));
            print!("\n{:<w$}", "decode   Mp/s", w = w_header);
            (0..n).for_each(|i| print!("{:>w$.1}", mpixels / total_decode_sec[i], w = w_col));
            print!("\n{:<w$}", "         MB/s", w = w_header);
            (0..n).for_each(|i| print!("{:>w$.1}", mbytes / total_decode_sec[i], w = w_col));
            print_sep("\n");
            // print!("\n{:<w$}", "         ms", w = w_header);
            // (0..n).for_each(|i| print!("{:>w$.2}", total_encode_sec[i] * 1e3, w = w_col));
            print!("\n{:<w$}", "encode   Mp/s", w = w_header);
            (0..n).for_each(|i| print!("{:>w$.1}", mpixels / total_encode_sec[i], w = w_col));
            print!("\n{:<w$}", "         MB/s", w = w_header);
            (0..n).for_each(|i| print!("{:>w$.1}", mbytes / total_encode_sec[i], w = w_col));
            print_sep("\n");
            println!();
        } else {
            let (w_name, w_col) = (9, 13);
            println!("---");
            print!("{:<w$}", "", w = w_name);
            // print!("{:>w$}", "decode:ms", w = w_col);
            // print!("{:>w$}", "encode:ms", w = w_col);
            print!("{:>w$}", "decode:Mp/s", w = w_col);
            print!("{:>w$}", "encode:Mp/s", w = w_col);
            print!("{:>w$}", "decode:MB/s", w = w_col);
            print!("{:>w$}", "encode:MB/s", w = w_col);
            println!();
            for (i, codec_name) in codec_names.iter().enumerate() {
                let decode_sec = total_decode_sec[i];
                let encode_sec = total_encode_sec[i];
                let (decode_mpps, encode_mpps) = (mpixels / decode_sec, mpixels / encode_sec);
                let (decode_mbps, encode_mbps) = (mbytes / decode_sec, mbytes / encode_sec);
                print!("{:<w$}", codec_name, w = w_name);
                // print!("{:>w$.2}", decode_sec * 1e3, w = w_col);
                // print!("{:>w$.2}", encode_sec * 1e3, w = w_col);
                print!("{:>w$.1}", decode_mpps, w = w_col);
                print!("{:>w$.1}", encode_mpps, w = w_col);
                print!("{:>w$.1}", decode_mbps, w = w_col);
                print!("{:>w$.1}", encode_mbps, w = w_col);
                println!();
            }
        }
    }
}

fn bench_png(filename: &Path, seconds: f64, use_median: bool, stream: bool) -> Result<ImageBench> {
    let f = filename.to_string_lossy();
    let img = Image::read_png(filename).context(format!("error reading PNG file: {}", f))?;
    let size_png_kb = fs::metadata(filename)?.len() / 1024;
    let size_mb_raw = img.n_bytes() as f64 / 1024. / 1024.;
    let mpixels = img.n_pixels() as f64 / 1e6;
    println!(
        "{} ({}x{}:{}, {} KB png, {:.2} MB raw, {:.2} MP)",
        f, img.width, img.height, img.channels, size_png_kb, size_mb_raw, mpixels
    );
    let mut bench = ImageBench::new(&img);
    bench.run(&CodecQoiC, &img, seconds)?;
    bench.run(&CodecQoiRust { stream }, &img, seconds)?;
    bench.report(use_median);
    Ok(bench)
}

fn bench_suite(
    files: &[PathBuf], seconds: f64, use_median: bool, fancy: bool, stream: bool,
) -> Result<()> {
    let mut totals = BenchTotals::new();
    for file in files {
        match bench_png(file, seconds, use_median, stream) {
            Ok(res) => totals.update(&res),
            Err(err) => eprintln!("{:?}", err),
        }
    }
    if totals.results.len() > 1 {
        totals.report(use_median, fancy);
    }
    Ok(())
}

#[derive(Debug, StructOpt)]
struct Args {
    /// Files or directories containing png images.
    #[structopt(parse(from_os_str))]
    paths: Vec<PathBuf>,
    /// Number of seconds allocated for each image/codec.
    #[structopt(short, long, default_value = "1")]
    seconds: f64,
    /// Use average (mean) instead of the median.
    #[structopt(short, long)]
    average: bool,
    /// Simple totals, no fancy tables.
    #[structopt(long)]
    simple: bool,
    /// Use stream API for qoi-rust.
    #[structopt(long)]
    stream: bool,
}

fn main() -> Result<()> {
    let args = <Args as StructOpt>::from_args();
    ensure!(!args.paths.is_empty(), "no input paths given");
    let files = find_pngs(&args.paths)?;
    ensure!(!files.is_empty(), "no PNG files found in given paths");
    bench_suite(&files, args.seconds, !args.average, !args.simple, args.stream)?;
    Ok(())
}
