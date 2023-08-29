pub trait HasName {
    type Name;
    fn name(&self) -> Self::Name;
}
