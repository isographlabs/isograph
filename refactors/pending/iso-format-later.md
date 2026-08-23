# Format iso literals, later

Requires iso-format.md. Independent of lsp-diagnostics.md.

iso-format.md is v1: `{` breaks and indents, descriptions own a line, commas between list items, `(` does not break. This file is the next shapes. No types, no consume catalog, no printer algorithm.

## What the user does

After v1, save of `user(arg: { b: true }, c: false)` still yields:

```
user(arg: {
  b: true
}, c: false)
```

The items below make that argument list break as well, and make long or nested lists break the same way.

## Features

- `(` breaks when something inside it already broke, so the example above becomes a broken argument list instead of `}, c: false)` on the close-brace line.
- `(` also breaks when the packed single line would be longer than some width.
- Once `(` breaks, arguments sit one per line (and the same for other currently-inline lists: variable lists, value lists).
- Selection items one per line when the `{` block is already breaking for a nested `{`, or when the packed `{ firstName, lastName, ... }` line is too long.
- Trailing commas on broken lists.
- Whether a space before `{` becomes a newline more often than the description exception.
