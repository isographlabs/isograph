use serde::Serialize;

#[macro_export]
macro_rules! derive_display {
    ($type:ident) => {
        impl std::fmt::Display for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0, f)
            }
        }
    };
}

#[derive(Debug, Serialize)]
pub struct QueryText(pub String);
derive_display!(QueryText);
