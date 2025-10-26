# isograph_compiler

This crate sets up batch mode and watch mode, which is to say it should:
- set up file system watchers,
- read and write from the file system, and
- call the "root query" (`get_artifact_path_and_content`)

It should not do much else!
