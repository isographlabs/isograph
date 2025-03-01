/// Distinguishes between server-defined items and locally-defined items.
///
/// For example, server fields and client fields.
#[derive(Debug, Clone, Copy, Ord, PartialOrd, PartialEq, Eq)]
pub enum DefinitionLocation<TServer, TClient> {
    Server(TServer),
    Client(TClient),
}

/// Distinguishes between items are are "scalar-like" and objects that
/// are "object-like". Examples include:
///
/// - client fields vs client pointers
/// - scalar field selections (i.e. those without selection sets) vs
///   linked field selections.
/// - schema scalars vs schema objects
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum SelectionType<TScalar, TObject> {
    Scalar(TScalar),
    Object(TObject),
}
