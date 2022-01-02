#[inline(always)]
#[cold]
pub const fn cold() {}

#[inline(always)]
#[allow(unused)]
pub const fn likely(b: bool) -> bool {
    if !b {
        cold();
    }
    b
}

#[inline(always)]
pub const fn unlikely(b: bool) -> bool {
    if b {
        cold();
    }
    b
}

pub trait Writer {
    type Output;

    fn write_one(self, v: u8) -> Self::Output;
    fn write_many(self, v: &[u8]) -> Self::Output;
}

impl<'a> Writer for BytesMut<'a> {
    type Output = BytesMut<'a>;

    #[inline]
    fn write_one(self, v: u8) -> Self {
        BytesMut::write_one(self, v)
    }

    #[inline]
    fn write_many(self, v: &[u8]) -> Self {
        BytesMut::write_many(self, v)
    }
}

pub struct BytesMut<'a>(&'a mut [u8]);

impl<'a> BytesMut<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self(buf)
    }

    #[inline]
    pub fn write_one(self, v: u8) -> Self {
        if let Some((first, tail)) = self.0.split_first_mut() {
            *first = v;
            Self(tail)
        } else {
            cold();
            panic!();
        }
    }

    #[inline]
    pub fn write_many(self, v: &[u8]) -> Self {
        let (head, tail) = self.0.split_at_mut(v.len());
        head.copy_from_slice(v);
        Self(tail)
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.0.len()
    }
}
