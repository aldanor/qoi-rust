use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_src = out_dir.join("qoi.c");
    fs::write(&out_src, "#include \"qoi.h\"\n").unwrap();

    cc::Build::new()
        .file(&out_src)
        .include("../ext/qoi")
        .define("QOI_NO_STDIO", None)
        .define("QOI_IMPLEMENTATION", None)
        .flag_if_supported("-Wno-unsequenced")
        .opt_level(3)
        .compile("qoi");
}
