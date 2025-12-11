# Troubleshooting

## Installation Issues

### `tree-sitter install failed`

This library depends on node-gyp to run. You may need to install it on your system. \
Fix: `pnpm add node-gyp`

### `tree-sitter-cli install script failed`

The node-gyp build pipeline requires C++20 or later. You may need to set the flag before building and compiling. \
Fix: `CXXFLAGS="-std=c++20" pnpm i`

## Runtime Issues

### ``build-compiler failed: feature `edition2024` is required``

The compiler needs at least Cargo 1.85 to run. You'll need to update your local version to at least that version or higher. \
Fix: `rustup update`
