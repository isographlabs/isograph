# Rust Compiler Guidelines

## Tooling & Verification
- LSP Integration: You have access to `rust-analyzer`. Before refactoring, use "Go to Definition" or "Hover" to confirm type signatures, and rely as much as possible on deterministic refactors (e.g. use rust analyzer to rename.)
- Diagnostics: Always check for compiler warnings/errors after writing code. Do not consider a task finished if `rust-analyzer` reports diagnostics.

## Codebase-Specific Patterns

### Marking Technical Debt

When taking shortcuts, use these patterns:

* `.note_todo("explanation")`: Preferred method for marking code that needs improvement. 
  ```rust
  some_value.clone().note_todo("Do not clone. Use a MemoRef.")
  ```

* `// TODO explanation`: Traditional comment for marking future work. Only use this when there is no value that can accept `.note_todo()`. Include what the correct solution should be.
  ```rust
  // TODO use the directives location, then the specific location
  ```

* `.note_do_not_commit("explanation")`: Used to mark code that absolutely must be fixed before landing. Clippy will reject code with this marker, preventing accidental commits of incomplete work.
  ```rust
  some_temporary_hack().note_do_not_commit("This breaks production, fix before merge")
  ```

* `// HACK: explanation`: Mark genuinely hacky code that works but shouldn't be done this way. Explain why it's suboptimal.
  ```rust
  // HACK: we're essentially splitting the field arg by . and keeping the same
  ```

### Location Handling

Attempt to avoid these:
* `todo_generated()`
* `Location::Generated`
* `Span::todo_generated()`
* `EmbeddedLocation::todo_generated()`

These are markers for incomplete location tracking. The correct solution is to thread actual source locations through the code. We do not always have great locations.

### Debugging

When debugging the compiler:

* `.dbg()`: Standard Rust debugging macro. Prints value and location to stderr.
* `.dbg_with_note("note")`: Custom macro for adding context to debug output. Useful when debugging complex compilation flows to understand which code path is executing.
  ```rust
  foo.dbg_with_note!("Processing entity in validation phase");
  ```

These are for development only. Do not commit debug statements to the codebase. Clippy will disallow it, anyway.
