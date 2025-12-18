/// A fancy wrapper around a String. Use this for errors that occur before we've
/// parsed things. (Locations don't exist at that point, and are created when
/// parsing.)
pub struct LocationFreeDiagnostic(pub String);

impl From<String> for LocationFreeDiagnostic {
    fn from(value: String) -> Self {
        LocationFreeDiagnostic(value)
    }
}

pub type LocationFreeDiagnosticResult<T> = Result<T, LocationFreeDiagnostic>;
pub type LocationFreeDiagnosticVecResult<T> = Result<T, Vec<LocationFreeDiagnostic>>;

impl std::fmt::Display for LocationFreeDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl LocationFreeDiagnostic {
    pub fn from_error(e: impl std::error::Error) -> Self {
        LocationFreeDiagnostic(e.to_string())
    }
}
