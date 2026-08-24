# Format iso literals, later

Requires iso-format.md. Independent of lsp-diagnostics.md.

Shapes after iso-format.md. iso-format.md always one item per line with a trailing comma, and always breaks `{` `(` and value `[`. When a group's packed single line is shorter than the width, it packs onto one line. A nested group that itself packs can sit on that line.

## What the user does

Save of `user(arg: { b: true })` yields `user(arg: { b: true, },)` when that packed line is under the width. Longer lists stay the broken form from iso-format.md.
