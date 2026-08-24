# Format iso literals, later

Requires iso-format.md. Independent of lsp-diagnostics.md.

Shapes after iso-format.md. iso-format.md already puts a comma after every list item and always breaks `{`. A `(` group whose interior already contains a line break, or whose packed single line is longer than the width, breaks: arguments (and variable lists, value lists) sit one per line. A `{` group whose interior already contains a nested broken `{`, or whose packed `{ firstName, lastName, ... }` line is longer than the width, puts each selection on its own line.

## What the user does

Save of `user(arg: { b: true }, c: false)` yields:

```
user(
  arg: {
    b: true,
  },
  c: false,
)
```
