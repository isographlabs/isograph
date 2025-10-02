use std::any::TypeId;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::ops::Deref;

use serde::Serialize;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WithSerialize<T>(pub T);

impl<T> WithSerialize<T> {
    #[inline]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for WithSerialize<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        &self.0
    }
}

struct HasherWrite<'a, H: Hasher>(&'a mut H);

impl<'a, H: Hasher> Write for HasherWrite<'a, H> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf);
        Ok(buf.len())
    }
    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<T: Serialize + 'static> Hash for WithSerialize<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        TypeId::of::<T>().hash(state);

        let config = bincode::config::standard()
            .with_little_endian()
            .with_fixed_int_encoding();

        let mut hw = HasherWrite(state);
        bincode::serde::encode_into_std_write(&self.0, &mut hw, config)
            .expect("cannot serialize inner value");
    }
}
