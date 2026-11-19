use intern::string_key::{Intern, Lookup};
use prelude::Postfix;
use std::borrow::Cow;
use std::path::{Path, PathBuf};

use span::Span;
pub use span::WithGenericLocation;

use crate::{CurrentWorkingDirectory, RelativePathToSourceFile};

/// A source, which consists of a filename, and an optional span
/// indicating the subset of the file which corresponds to the
/// source.
///
/// TODO consider whether to replace the span with an index,
/// as this will probably mean that sources are more reusable
/// during watch mode.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TextSource {
    pub relative_path_to_source_file: RelativePathToSourceFile,
    pub span: Option<Span>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EmbeddedLocation {
    pub text_source: TextSource,
    /// The span is relative to the Source's span, not to the
    /// entire source file.
    pub span: Span,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, derive_more::From)]
pub enum Location {
    #[from]
    Embedded(EmbeddedLocation),
    Generated,
}

impl Location {
    pub fn new(text_source: TextSource, span: Span) -> Self {
        Location::Embedded(EmbeddedLocation::new(text_source, span))
    }

    pub fn as_embedded_location(self) -> Option<EmbeddedLocation> {
        match self {
            Location::Embedded(embedded_location) => embedded_location.wrap_some(),
            Location::Generated => None,
        }
    }
}
impl EmbeddedLocation {
    pub fn new(text_source: TextSource, span: Span) -> Self {
        EmbeddedLocation { text_source, span }
    }
}

pub type WithLocation<TItem> = WithGenericLocation<TItem, Location>;

pub trait WithLocationPostfix
where
    Self: Sized,
{
    fn with_generated_location(self) -> WithLocation<Self> {
        WithLocation::new(self, Location::Generated)
    }

    fn with_no_location(self) -> WithNoLocation<Self> {
        WithGenericLocation::new(self, ())
    }

    fn with_location<TLocation>(self, item: TLocation) -> WithGenericLocation<Self, TLocation> {
        WithGenericLocation::new(self, item)
    }

    fn with_missing_location<TLocation>(self) -> WithGenericLocation<Self, Option<TLocation>> {
        WithGenericLocation::new(self, None)
    }

    fn with_some_location<TLocation>(
        self,
        location: TLocation,
    ) -> WithGenericLocation<Self, Option<TLocation>> {
        WithGenericLocation::new(self, location.wrap_some())
    }
}

pub type WithOptionalLocation<TItem> = WithGenericLocation<TItem, Option<EmbeddedLocation>>;
pub type WithEmbeddedLocation<TItem> = WithGenericLocation<TItem, EmbeddedLocation>;

impl<T> WithLocationPostfix for T {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum RelativePathError {
    #[error("the path has no form relative to the working directory")]
    NotDiffable,
    #[error("the relative path is not UTF-8")]
    NotUtf8,
}

pub fn relative_path_from_absolute_and_working_directory(
    current_working_directory: CurrentWorkingDirectory,
    absolute_path: &Path,
) -> Result<RelativePathToSourceFile, RelativePathError> {
    let cwd = PathBuf::from(current_working_directory.lookup());
    let relative = pathdiff::diff_paths(
        without_windows_verbatim_prefix(absolute_path),
        without_windows_verbatim_prefix(cwd.as_path()),
    )
    .ok_or(RelativePathError::NotDiffable)?;
    relative
        .to_str()
        .ok_or(RelativePathError::NotUtf8)?
        .intern()
        .to::<RelativePathToSourceFile>()
        .wrap_ok()
}

fn without_windows_verbatim_prefix(path: &Path) -> Cow<'_, Path> {
    match strip_windows_verbatim_prefix(path) {
        Some(stripped) => Cow::Owned(stripped),
        None => Cow::Borrowed(path),
    }
}

fn strip_windows_verbatim_prefix(path: &Path) -> Option<PathBuf> {
    let text = path.to_str()?;
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        let mut unc = String::from(r"\\");
        unc.push_str(rest);
        return PathBuf::from(unc).wrap_some();
    }
    text.strip_prefix(r"\\?\").map(PathBuf::from)
}

pub type WithNoLocation<TItem> = WithGenericLocation<TItem, ()>;

#[cfg(test)]
mod tests {
    use intern::string_key::{Intern, Lookup};
    use prelude::Postfix;
    use std::path::PathBuf;

    use super::{RelativePathError, relative_path_from_absolute_and_working_directory};
    use crate::CurrentWorkingDirectory;

    fn cwd(s: &str) -> CurrentWorkingDirectory {
        s.intern().to()
    }

    #[cfg(not(windows))]
    #[test]
    fn a_file_under_the_working_directory_is_the_relative_remainder() {
        let relative = relative_path_from_absolute_and_working_directory(
            cwd("/tmp/proj"),
            &PathBuf::from("/tmp/proj/src/a.ts"),
        )
        .expect("the test path is under the working directory");
        assert_eq!(relative.lookup(), "src/a.ts");
    }

    #[cfg(windows)]
    #[test]
    fn a_verbatim_working_directory_and_a_disk_path_share_the_relative_remainder() {
        let relative = relative_path_from_absolute_and_working_directory(
            cwd(r"\\?\C:\proj"),
            &PathBuf::from(r"C:\proj\src\Home.ts"),
        )
        .expect("the test path is under the working directory");
        assert_eq!(relative.lookup(), r"src\Home.ts");
    }

    #[cfg(windows)]
    #[test]
    fn a_disk_working_directory_and_a_verbatim_path_share_the_relative_remainder() {
        let relative = relative_path_from_absolute_and_working_directory(
            cwd(r"C:\proj"),
            &PathBuf::from(r"\\?\C:\proj\src\Home.ts"),
        )
        .expect("the test path is under the working directory");
        assert_eq!(relative.lookup(), r"src\Home.ts");
    }

    #[cfg(windows)]
    #[test]
    fn a_verbatim_unc_working_directory_and_a_unc_path_share_the_relative_remainder() {
        let relative = relative_path_from_absolute_and_working_directory(
            cwd(r"\\?\UNC\server\share"),
            &PathBuf::from(r"\\server\share\src\Home.ts"),
        )
        .expect("the test path is under the working directory");
        assert_eq!(relative.lookup(), r"src\Home.ts");
    }

    #[cfg(not(windows))]
    #[test]
    fn a_relative_path_against_an_absolute_working_directory_is_not_diffable() {
        assert_eq!(
            relative_path_from_absolute_and_working_directory(
                cwd("/tmp/proj"),
                &PathBuf::from("src/a.ts"),
            ),
            RelativePathError::NotDiffable.wrap_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_path_with_a_non_utf8_component_is_not_utf8() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        assert_eq!(
            relative_path_from_absolute_and_working_directory(
                cwd("/tmp/proj"),
                &PathBuf::from(OsStr::from_bytes(b"/tmp/proj/\xFF.ts")),
            ),
            RelativePathError::NotUtf8.wrap_err()
        );
    }
}
