#[inline(always)]
#[cold]
pub const fn cold() {}

#[inline(always)]
pub const fn likely(b: bool) -> bool {
    if !b {
        cold()
    }
    b
}

#[inline(always)]
pub const fn unlikely(b: bool) -> bool {
    if b {
        cold()
    }
    b
}
