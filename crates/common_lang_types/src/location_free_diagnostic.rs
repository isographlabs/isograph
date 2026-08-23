/// A fancy wrapper around a String. Use this for errors that occur before we've
/// parsed things. (Locations don't exist at that point, and are created when
/// parsing.)
#[derive(derive_more::From, derive_more::Display)]
pub struct LocationFreeDiagnostic(pub String);

pub type LocationFreeDiagnosticResult<T> = Result<T, LocationFreeDiagnostic>;
pub type LocationFreeDiagnosticVecResult<T> = Result<T, Vec<LocationFreeDiagnostic>>;

impl LocationFreeDiagnostic {
    pub fn from_error(e: impl std::error::Error) -> Self {
        LocationFreeDiagnostic(e.to_string())
    }
}
