pub trait Postfix
where
    Self: Sized,
{
    #[inline(always)]
    fn wrap<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }

    #[inline(always)]
    fn wrap_ok<E>(self) -> Result<Self, E> {
        Ok(self)
    }

    #[inline(always)]
    fn wrap_err<T>(self) -> Result<T, Self> {
        Err(self)
    }

    #[inline(always)]
    fn wrap_some(self) -> Option<Self> {
        Some(self)
    }

    #[inline(always)]
    fn boxed(self) -> Box<Self> {
        Box::new(self)
    }

    #[inline(always)]
    fn to<T>(self) -> T
    where
        Self: Into<T>,
    {
        self.into()
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
