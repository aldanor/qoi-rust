use std::cmp::Ordering;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{bail, ensure, Context, Result};
use bytemuck::cast_slice;
use c_vec::CVec;
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
}

trait Codec {
    type Output: AsRef<[u8]>;

    fn name() -> &'static str;
    fn encode(img: &Image) -> Result<Self::Output>;
    fn decode(data: &[u8], img: &Image) -> Result<Self::Output>;
}

struct CodecQoiFast;

impl Codec for CodecQoiFast {
    type Output = Vec<u8>;

    fn name() -> &'static str {
        "qoi-fast"
    }

    fn encode(img: &Image) -> Result<Vec<u8>> {
        Ok(qoi_fast::encode_to_vec(&img.data, img.width, img.height)?)
    }

    fn decode(data: &[u8], _img: &Image) -> Result<Vec<u8>> {
        Ok(qoi_fast::decode_to_vec(data)?.1)
    }
}

struct CodecQoiC;

impl Codec for CodecQoiC {
    type Output = CVec<u8>;

    fn name() -> &'static str {
        "qoi.h"
    }

    fn encode(img: &Image) -> Result<CVec<u8>> {
        libqoi::qoi_encode(&img.data, img.width, img.height, img.channels)
    }

    fn decode(data: &[u8], img: &Image) -> Result<CVec<u8>> {
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
}

impl ImageBench {
    pub fn new(img: &Image) -> Self {
        Self { results: vec![], n_pixels: img.n_pixels() }
    }

    pub fn run<C: Codec>(&mut self, img: &Image, sec_allowed: f64) -> Result<()> {
        let (encoded, t_encode) = timeit(|| C::encode(img));
        let encoded = encoded?;
        let (decoded, t_decode) = timeit(|| C::decode(encoded.as_ref(), img));
        let decoded = decoded?;
        let roundtrip = decoded.as_ref() == img.data.as_slice();
        if C::name() == "qoi-fast" {
            assert!(roundtrip, "{}: decoded data doesn't roundtrip", C::name());
        } else {
            ensure!(roundtrip, "{}: decoded data doesn't roundtrip", C::name());
        }

        let n_encode = (sec_allowed / 2. / t_encode.as_secs_f64()).max(2.).ceil() as usize;
        let mut encode_tm = Vec::with_capacity(n_encode);
        for _ in 0..n_encode {
            encode_tm.push(timeit(|| C::encode(img)).1);
        }
        let encode_sec = encode_tm.iter().map(Duration::as_secs_f64).collect();

        let n_decode = (sec_allowed / 2. / t_decode.as_secs_f64()).max(2.).ceil() as usize;
        let mut decode_tm = Vec::with_capacity(n_decode);
        for _ in 0..n_decode {
            decode_tm.push(timeit(|| C::decode(encoded.as_ref(), img)).1);
        }
        let decode_sec = decode_tm.iter().map(Duration::as_secs_f64).collect();

        self.results.push(BenchResult::new(C::name(), decode_sec, encode_sec));
        Ok(())
    }

    pub fn report(&self, use_median: bool) {
        let (w_name, w_col) = (11, 13);
        print!("{:<w$}", "", w = w_name);
        print!("{:>w$}", "decode:ms", w = w_col);
        print!("{:>w$}", "encode:ms", w = w_col);
        print!("{:>w$}", "decode:mp/s", w = w_col);
        print!("{:>w$}", "encode:mp/s", w = w_col);
        println!();
        for r in &self.results {
            let decode_sec = r.average_decode_sec(use_median);
            let encode_sec = r.average_encode_sec(use_median);
            let mpixels = self.n_pixels as f64 / 1e6;
            let (decode_mpps, encode_mpps) = (mpixels / decode_sec, mpixels / encode_sec);

            print!("{:<w$}", r.codec, w = w_name);
            print!("{:>w$.2}", decode_sec * 1e3, w = w_col);
            print!("{:>w$.2}", encode_sec * 1e3, w = w_col);
            print!("{:>w$.1}", decode_mpps, w = w_col);
            print!("{:>w$.1}", encode_mpps, w = w_col);
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

    pub fn report(&self, use_median: bool) {
        if self.results.is_empty() {
            return;
        }
        let codec_names: Vec<_> = self.results[0].results.iter().map(|r| r.codec.clone()).collect();
        let n_codecs = codec_names.len();
        let (mut total_decode_sec, mut total_encode_sec, mut total_size) =
            (vec![0.; n_codecs], vec![0.; n_codecs], 0);
        for r in &self.results {
            total_size += r.n_pixels;
            for i in 0..n_codecs {
                // sum of medians is not the median of sums, but w/e, good enough here
                total_decode_sec[i] += r.results[i].average_decode_sec(use_median);
                total_encode_sec[i] += r.results[i].average_encode_sec(use_median);
            }
        }

        let (w_name, w_col) = (11, 13);
        println!("---");
        println!(
            "Overall results: ({} images, {:.1} MB):",
            self.results.len(),
            total_size as f64 / 1024. / 1024.
        );
        println!("---");
        print!("{:<w$}", "", w = w_name);
        print!("{:>w$}", "decode:ms", w = w_col);
        print!("{:>w$}", "encode:ms", w = w_col);
        print!("{:>w$}", "decode:mp/s", w = w_col);
        print!("{:>w$}", "encode:mp/s", w = w_col);
        println!();
        for (i, codec_name) in codec_names.iter().enumerate() {
            let decode_sec = total_decode_sec[i];
            let encode_sec = total_encode_sec[i];
            let mpixels = total_size as f64 / 1e6;
            let (decode_mpps, encode_mpps) = (mpixels / decode_sec, mpixels / encode_sec);

            print!("{:<w$}", codec_name, w = w_name);
            print!("{:>w$.2}", decode_sec * 1e3, w = w_col);
            print!("{:>w$.2}", encode_sec * 1e3, w = w_col);
            print!("{:>w$.1}", decode_mpps, w = w_col);
            print!("{:>w$.1}", encode_mpps, w = w_col);
            println!();
        }
    }
}

fn bench_png(filename: &Path, seconds: f64, use_median: bool) -> Result<ImageBench> {
    let f = filename.to_string_lossy();
    let img = Image::read_png(filename).context(format!("error reading PNG file: {}", f))?;
    let size_kb = fs::metadata(filename)?.len() / 1024;
    let mpixels = img.n_pixels() as f64 / 1e6;
    println!(
        "{} ({}x{}:{}, {} KB, {:.2}MP)",
        f, img.width, img.height, img.channels, size_kb, mpixels
    );
    let mut bench = ImageBench::new(&img);
    bench.run::<CodecQoiC>(&img, seconds)?;
    bench.run::<CodecQoiFast>(&img, seconds)?;
    bench.report(use_median);
    Ok(bench)
}

fn bench_suite(files: &[PathBuf], seconds: f64, use_median: bool) -> Result<()> {
    let mut totals = BenchTotals::new();
    for file in files {
        match bench_png(file, seconds, use_median) {
            Ok(res) => totals.update(&res),
            Err(err) => eprintln!("{:?}", err),
        }
    }
    if totals.results.len() > 1 {
        totals.report(use_median);
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
}

fn main() -> Result<()> {
    let args = <Args as StructOpt>::from_args();
    ensure!(!args.paths.is_empty(), "no input paths given");
    let files = find_pngs(&args.paths)?;
    ensure!(!files.is_empty(), "no PNG files found in given paths");
    bench_suite(&files, args.seconds, !args.average)?;
    Ok(())
}
