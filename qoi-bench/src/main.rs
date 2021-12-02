use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::ptr;
use std::time::{Duration, Instant};

use anyhow::{bail, ensure, Context, Result};
use libc::{c_int, c_void};
use structopt::StructOpt;
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Copy, Clone, Default)]
#[repr(C)]
#[allow(non_camel_case_types)]
struct qoi_desc {
    width: u32,
    height: u32,
    channels: u8,
    colorspace: u8,
}

extern "C" {
    fn qoi_encode(data: *const c_void, desc: *const qoi_desc, out_len: *mut c_int) -> *mut c_void;
    fn qoi_decode(
        data: *const c_void, size: c_int, desc: *mut qoi_desc, channels: c_int,
    ) -> *mut c_void;
}

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
    Ok(out)
}

struct Image {
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub data: Vec<u8>,
}

impl Image {
    pub const fn n_pixels(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }
}

fn read_png(filename: &Path) -> Result<Image> {
    let decoder = png::Decoder::new(File::open(filename)?);
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf)?;
    let bytes = &buf[..info.buffer_size()];
    Ok(Image {
        width: info.width,
        height: info.height,
        channels: info.color_type.samples() as u8,
        data: bytes.to_vec(),
    })
}

trait Codec {
    fn name() -> &'static str;

    fn encode(img: &Image) -> Result<Vec<u8>>;

    fn encode_bench(img: &Image) -> Result<()> {
        let _ = black_box(Self::encode(img)?);
        Ok(())
    }

    fn decode(data: &[u8], img: &Image) -> Result<Vec<u8>>;

    fn decode_bench(data: &[u8], img: &Image) -> Result<()> {
        let _ = black_box(Self::decode(data, img)?);
        Ok(())
    }
}

struct CodecQoiFast;

impl Codec for CodecQoiFast {
    fn name() -> &'static str {
        "qoi-fast"
    }

    fn encode(img: &Image) -> Result<Vec<u8>> {
        Ok(qoi_fast::qoi_encode_to_vec(&img.data, img.width, img.height, img.channels, 0)?)
    }

    fn decode(data: &[u8], img: &Image) -> Result<Vec<u8>> {
        Ok(qoi_fast::qoi_decode_to_vec(data, img.channels)?.1)
    }
}

struct CodecQoiFastCanonical;

impl Codec for CodecQoiFastCanonical {
    fn name() -> &'static str {
        "qoi-fast(c)"
    }

    fn encode(img: &Image) -> Result<Vec<u8>> {
        Ok(qoi_fast::canonical::qoi_encode_to_vec(
            &img.data,
            img.width,
            img.height,
            img.channels,
            0,
        )?)
    }

    fn decode(data: &[u8], img: &Image) -> Result<Vec<u8>> {
        Ok(qoi_fast::qoi_decode_to_vec(data, img.channels)?.1)
    }
}

struct CodecQoiC;

impl CodecQoiC {
    unsafe fn qoi_encode(img: &Image) -> Result<(*mut u8, usize)> {
        let desc = qoi_desc {
            width: img.width,
            height: img.height,
            channels: img.channels,
            colorspace: 0,
        };
        let mut out_len: c_int = 0;
        let ptr =
            qoi_encode(img.data.as_ptr() as *const _, &desc as *const _, &mut out_len as *mut _);
        ensure!(!ptr.is_null(), "error encoding with qoi-c");
        Ok((ptr as _, out_len as _))
    }

    unsafe fn qoi_decode(data: &[u8], img: &Image) -> Result<(*mut u8, qoi_desc)> {
        let mut desc = qoi_desc::default();
        let ptr =
            qoi_decode(data.as_ptr() as _, data.len() as _, &mut desc as *mut _, img.channels as _);
        ensure!(!ptr.is_null(), "error decoding with qoi-c");
        Ok((ptr as _, desc))
    }
}

impl Codec for CodecQoiC {
    fn name() -> &'static str {
        "qoi-c"
    }

    fn encode(img: &Image) -> Result<Vec<u8>> {
        unsafe {
            let (ptr, len) = Self::qoi_encode(img)?;
            let mut vec = Vec::with_capacity(len);
            vec.set_len(len);
            ptr::copy_nonoverlapping(ptr, vec.as_mut_ptr(), len);
            libc::free(ptr as _);
            Ok(vec)
        }
    }

    fn encode_bench(img: &Image) -> Result<()> {
        unsafe {
            let (ptr, _) = Self::qoi_encode(img)?;
            libc::free(ptr as _);
            Ok(())
        }
    }

    fn decode(data: &[u8], img: &Image) -> Result<Vec<u8>> {
        unsafe {
            let (ptr, desc) = Self::qoi_decode(data, img)?;
            let len = desc.width as usize * desc.height as usize * desc.channels as usize;
            let mut vec = Vec::with_capacity(len);
            vec.set_len(len);
            ptr::copy_nonoverlapping(ptr, vec.as_mut_ptr(), len);
            libc::free(ptr as _);
            Ok(vec)
        }
    }

    fn decode_bench(data: &[u8], img: &Image) -> Result<()> {
        unsafe {
            let (ptr, _) = Self::qoi_decode(data, img)?;
            libc::free(ptr as _);
            Ok(())
        }
    }
}

struct BenchResult {
    pub codec: String,
    pub encode_sec: Vec<f64>,
    pub decode_sec: Vec<f64>,
    pub size_encoded: usize,
}

struct ImageBench {
    img: Image,
    sec_allowed: f64,
    results: Vec<BenchResult>,
}

impl ImageBench {
    pub fn new(img: Image, sec_allowed: f64) -> Self {
        Self { img, sec_allowed, results: vec![] }
    }

    pub fn run<C: Codec>(&mut self) -> Result<()> {
        let (encoded, t_encode) = timeit(|| C::encode(&self.img));
        let encoded = encoded?;
        let (decoded, t_decode) = timeit(|| C::decode(&encoded, &self.img));
        let decoded = decoded?;
        ensure!(decoded.as_slice() == self.img.data.as_slice(), "decoded data doesn't roundtrip");

        let n_encode = (self.sec_allowed / 2. / t_encode.as_secs_f64()).max(2.).ceil() as usize;
        let mut encode_tm = Vec::with_capacity(n_encode);
        for _ in 0..n_encode {
            encode_tm.push(timeit(|| C::encode_bench(&self.img)).1);
        }
        encode_tm.sort_unstable();
        let encode_sec = encode_tm.iter().map(Duration::as_secs_f64).collect();

        let n_decode = (self.sec_allowed / 2. / t_decode.as_secs_f64()).max(2.).ceil() as usize;
        let mut decode_tm = Vec::with_capacity(n_decode);
        for _ in 0..n_decode {
            decode_tm.push(timeit(|| C::decode_bench(&encoded, &self.img)).1);
        }
        decode_tm.sort_unstable();
        let decode_sec = decode_tm.iter().map(Duration::as_secs_f64).collect();

        self.results.push(BenchResult {
            codec: C::name().to_owned(),
            encode_sec,
            decode_sec,
            size_encoded: encoded.len(),
        });
        Ok(())
    }

    pub fn report(&self, use_median: bool) {
        let (w_name, w_col) = (11, 13);
        print!("{:<w$}", "codec", w = w_name);
        print!("{:>w$}", "decode:ms", w = w_col);
        print!("{:>w$}", "encode:ms", w = w_col);
        print!("{:>w$}", "decode:mp/s", w = w_col);
        print!("{:>w$}", "encode:mp/s", w = w_col);
        print!("{:>w$}", "compression", w = w_col);
        print!("{:>w$}", "output:kb", w = w_col);
        println!();
        for r in &self.results {
            let (decode_sec, encode_sec) = if use_median {
                (r.decode_sec[r.decode_sec.len() / 2], r.encode_sec[r.encode_sec.len() / 2])
            } else {
                (mean(&r.decode_sec), mean(&r.encode_sec))
            };
            let mpixels = self.img.n_pixels() as f64 / 1e6;
            let (decode_mpps, encode_mpps) = (mpixels / decode_sec, mpixels / encode_sec);
            let comp_ratio_pct = r.size_encoded as f64 / self.img.data.len() as f64 * 1e2;
            let size_kb = r.size_encoded as f64 / 1024.;

            print!("{:<w$}", r.codec, w = w_name);
            print!("{:>w$.2}", decode_sec * 1e3, w = w_col);
            print!("{:>w$.2}", encode_sec * 1e3, w = w_col);
            print!("{:>w$.1}", decode_mpps, w = w_col);
            print!("{:>w$.1}", encode_mpps, w = w_col);
            print!("{:>w$.2}%", comp_ratio_pct, w = w_col - 1);
            print!("{:>w$.1}", size_kb, w = w_col);
            println!();
        }
    }
}

fn bench_png(filename: &Path) -> Result<()> {
    let f = filename.to_string_lossy();
    let img = read_png(filename).context(format!("error reading PNG file: {}", f))?;
    let size_kb = fs::metadata(filename)?.len() / 1024;
    let mpixels = img.n_pixels() as f64 / 1e6;
    println!(
        "{} ({}x{}:{}, {} KB, {:.1}MP)",
        f, img.width, img.height, img.channels, size_kb, mpixels
    );
    let mut bench = ImageBench::new(img, 5.);
    bench.run::<CodecQoiC>()?;
    bench.run::<CodecQoiFast>()?;
    bench.run::<CodecQoiFastCanonical>()?;
    bench.report(true);
    Ok(())
}

#[derive(Debug, StructOpt)]
struct Args {
    #[structopt(parse(from_os_str))]
    paths: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let args = <Args as StructOpt>::from_args();
    ensure!(!args.paths.is_empty(), "no input paths given");
    let files = find_pngs(&args.paths)?;
    ensure!(!files.is_empty(), "no PNG files found in given paths");
    for file in &files {
        bench_png(file)?;
    }
    Ok(())
}
