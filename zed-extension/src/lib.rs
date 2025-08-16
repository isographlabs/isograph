use zed_extension_api as zed;

struct IsographExtension;

impl zed::Extension for IsographExtension {
    fn new() -> Self {
        IsographExtension
    }
}

zed::register_extension!(IsographExtension);
