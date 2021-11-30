#[inline(always)]
#[allow(unused)]
pub const fn likely(b: bool) -> bool {
    // borrowed from `likely_stable` crate
    #[allow(clippy::needless_bool)]
    if (1i32).checked_div(if b { 1 } else { 0 }).is_some() {
        true
    } else {
        false
    }
}

#[inline(always)]
#[allow(unused)]
pub const fn unlikely(b: bool) -> bool {
    // borrowed from `likely_stable` crate
    #[allow(clippy::needless_bool)]
    if (1i32).checked_div(if b { 0 } else { 1 }).is_none() {
        true
    } else {
        false
    }
}
