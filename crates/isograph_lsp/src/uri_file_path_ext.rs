use std::path::PathBuf;

pub trait UriFilePathExt {
    fn to_file_path(&self) -> Result<PathBuf, ()>;
}

impl UriFilePathExt for lsp_types::Uri {
    fn to_file_path(&self) -> Result<PathBuf, ()> {
        let mut file_path = url::Url::parse(self.as_str())
            .map_err(|_| ())?
            .to_file_path();

        if cfg!(target_os = "windows") {
            file_path = std::fs::canonicalize(file_path?).map_err(|_| ());
        }

        file_path
    }
}
