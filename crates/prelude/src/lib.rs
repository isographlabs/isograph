pub trait Postfix
where
    Self: Sized,
{
    fn ok<E>(self) -> Result<Self, E> {
        Ok(self)
    }

    fn err<T>(self) -> Result<T, Self> {
        Err(self)
    }

    fn some(self) -> Option<Self> {
        Some(self)
    }
}

impl<T> Postfix for T {}

// TODO postfix debug
// TODO prevent that method ending up in main via clippy lint
