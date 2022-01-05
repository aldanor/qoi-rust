use anyhow::{ensure, Result};
use c_vec::CVec;
use libc::{c_int, c_void};

mod ffi {
    use libc::{c_int, c_uchar, c_uint, c_void};

    #[derive(Debug, Copy, Clone, Default)]
    #[repr(C)]
    #[allow(non_camel_case_types)]
    pub struct qoi_desc {
        pub width: c_uint,
        pub height: c_uint,
        pub channels: c_uchar,
        pub colorspace: c_uchar,
    }

    extern "C" {
        pub fn qoi_encode(
            data: *const c_void, desc: *const qoi_desc, out_len: *mut c_int,
        ) -> *mut c_void;
        pub fn qoi_decode(
            data: *const c_void, size: c_int, desc: *mut qoi_desc, channels: c_int,
        ) -> *mut c_void;
    }
}

pub use ffi::qoi_desc;

pub fn qoi_encode(data: &[u8], width: u32, height: u32, channels: u8) -> Result<CVec<u8>> {
    let desc =
        qoi_desc { width: width as _, height: height as _, channels: channels as _, colorspace: 0 };
    let mut out_len: c_int = 0;
    let out_ptr =
        unsafe { ffi::qoi_encode(data.as_ptr().cast(), &desc as *const _, &mut out_len as *mut _) };
    ensure!(!out_ptr.is_null(), "qoi.h: qoi_encode() returned null pointer");
    ensure!(out_len > 0, "qoi.h: qoi_encode() returned non-positive out_len");
    let vec = unsafe {
        CVec::new_with_dtor(out_ptr.cast::<u8>(), out_len as _, |p| libc::free(p.cast::<c_void>()))
    };
    Ok(vec)
}

pub fn qoi_decode(data: &[u8], channels: u8) -> Result<(qoi_desc, CVec<u8>)> {
    let mut desc = qoi_desc::default();
    let out_ptr = unsafe {
        ffi::qoi_decode(data.as_ptr() as _, data.len() as _, &mut desc as *mut _, channels as _)
    };
    ensure!(!out_ptr.is_null(), "qoi.h: qoi_decode() returned null pointer");
    let out_len = (desc.width as usize)
        .saturating_mul(desc.height as usize)
        .saturating_mul(channels as usize);
    let vec = unsafe {
        CVec::new_with_dtor(out_ptr.cast::<u8>(), out_len, |p| libc::free(p.cast::<c_void>()))
    };
    Ok((desc, vec))
}
