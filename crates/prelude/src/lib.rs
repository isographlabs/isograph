pub trait Postfix
where
    Self: Sized,
{
    #[inline(always)]
    fn ok<E>(self) -> Result<Self, E> {
        Ok(self)
    }

    #[inline(always)]
    fn err<T>(self) -> Result<T, Self> {
        Err(self)
    }

    #[inline(always)]
    fn some(self) -> Option<Self> {
        Some(self)
    }

    #[inline(always)]
    fn boxed(self) -> Box<Self> {
        Box::new(self)
    }

    #[inline(always)]
    fn dbg(self) -> Self
    where
        Self: std::fmt::Debug,
    {
        dbg!(self)
    }
}

impl<T> Postfix for T {}
