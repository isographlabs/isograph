# Convert `path` and `LineChar` to `LiteralId`

Requires memoized-parse-iso-literal.md (landed). Public callers still pass `path` and `LineChar`. The first intern turns that pair into `LiteralId`: the file plus the 0-based index of the literal in `THostLanguage::extract_iso_literals`. Extraction and parse are not stored at `(path, LineChar)`.

```text
literal_id_at_location(path, LineChar)
  -> THostLanguage::extract_iso_literals(path)
  + find_iso_literal_index(LineChar, file text, extract vec)

iso_literal_extraction(LiteralId)
  -> THostLanguage::extract_iso_literals(path)
  + vec[index]

parsed_iso_literal(text)
  -> parse_iso_literal(&str)
```

Hover, goto, and completion take `path` and `LineChar`. Inside, they call `literal_id_at_location`. That intern re-invokes per `LineChar`. The stored value is `LiteralId`. Then `iso_literal_extraction` of that id is one slot per literal in the file. `parsed_iso_literal` of the text is one slot.

Origin of the LineChar walk: `iso_literal_extraction` / `find_iso_literal_extraction` in `crates/isograph_compiler/src/iso_literals.rs` (extract-iso-literals-from-file.md). Origin of the parse intern: `parsed_iso_literal` (memoized-parse-iso-literal.md). Delta: `find` returns the vec index, not `&IsoLiteralExtraction`; `iso_literal_extraction` is keyed on `LiteralId`, not on `LineChar`; `iso_literal_text_at_location` and `parsed_iso_literal_at_location` are deleted. Every memo takes `&IsographState<THostLanguage>`.

`iso_literal_extraction(path, LineChar)` clones `IsoLiteralExtraction` (the text string, start index, context) into every `(path, LineChar)` slot. `parsed_iso_literal_at_location` clones the parse tree into every such slot. `LiteralId` is a `PathBuf` and a `usize`. That is the value stored per `LineChar`.

`parsed_iso_literal` is unchanged. file-semantic-tokens.md and `file_literals` already call extract-all and `parsed_iso_literal(text)`. They do not use the deleted memos.

Two shippable changes: `LiteralId` and the rekeyed extraction, then delete the `(path, LineChar)` text and tree memos.

## What the user does

No user-facing change. Tests intern a `DiskFile`, take a `LineChar` inside a literal, call `literal_id_at_location`, and assert `LiteralId { path, index: 0 }`. A second literal's interior is `index: 1`. Parse of that literal is `parsed_iso_literal` of `iso_literal_extraction(id).iso_literal_text`.

## Change 1: `LiteralId` and `iso_literal_extraction(LiteralId)`

```rust
// from crates/isograph_compiler/src/iso_literals.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LiteralId {
    pub path: PathBuf,
    pub index: usize,
}
```

`index` is the 0-based position in the `Vec` returned by `THostLanguage::extract_iso_literals` for `path`. Left to right, regex order. Not a byte offset. Not `iso_literal_start_index`.

pico intern requires `Hash + Clone` on an owned param. pico re-invoke compares `LiteralId` with `==`.

```rust
// from crates/isograph_compiler/src/iso_literals.rs
#[memo]
pub fn literal_id_at_location<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: PathBuf,
    line_char: LineChar,
) -> Option<LiteralId> {
    let extractions = THostLanguage::extract_iso_literals(db, path.clone()).as_ref()?;
    let source_id = db.get_disk_file_map().untracked().0.get(&path).copied()?;
    let file_content = db.get(source_id).contents.reference();
    let index = find_iso_literal_index(line_char, file_content, extractions)?;
    LiteralId { path, index }.wrap_some()
}

#[memo]
pub fn iso_literal_extraction<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    literal_id: LiteralId,
) -> Option<IsoLiteralExtraction<THostLanguage>> {
    let extractions =
        THostLanguage::extract_iso_literals(db, literal_id.path.clone()).as_ref()?;
    extractions.get(literal_id.index).cloned()
}
```

Delete the current `iso_literal_extraction(db, path, line_char)`. pico lookup of `literal_id_at_location` is `&Option<LiteralId>`. pico lookup of `iso_literal_extraction` is `&Option<IsoLiteralExtraction<THostLanguage>>`. `THostLanguage` is inferred from `db`.

`None` from `literal_id_at_location` is no `DiskFile`, or a `LineChar` that is not inside any literal text (the JS around the literals, including `iso(` and the closing backtick). `None` from `iso_literal_extraction` is no `DiskFile`, or `index` past the extract vec (the literal at that index is gone).

Before, `iso_literal_extraction` cloned the extraction at `(path, LineChar)` and also read the file to run `find`. After, `literal_id_at_location` runs `find` and stores `LiteralId`. `iso_literal_extraction` only indexes the extract vec. It does not read file text.

`find_iso_literal_extraction` is renamed `find_iso_literal_index`. Origin of the walk: the current function, verbatim except `enumerate` and the return.

```rust
// from crates/isograph_compiler/src/iso_literals.rs
fn find_iso_literal_index<THostLanguage: HostLanguage>(
    target_line_char: LineChar,
    file_content: &str,
    extracted_items: &[IsoLiteralExtraction<THostLanguage>],
) -> Option<usize> {
    let mut last_iteration_end_line_count = 0;
    let mut last_iteration_end_char_count = 0;
    let mut max_prev_span_end = 0;
    for (index, extract_item) in extracted_items.iter().enumerate() {
        let iso_literal_start_index = extract_item.iso_literal_start_index;
        let iso_literal_end_index = iso_literal_start_index + extract_item.iso_literal_text.len();

        let intermediate_content = &file_content[max_prev_span_end..iso_literal_start_index];
        let (intermediate_line, intermediate_char) = line_and_byte(intermediate_content);

        let start_line_count = last_iteration_end_line_count + intermediate_line;
        let start_char_count = if intermediate_line > 0 {
            intermediate_char
        } else {
            last_iteration_end_char_count + intermediate_char
        };

        let iso_content = &file_content[iso_literal_start_index..iso_literal_end_index];
        let (iso_line, iso_char) = line_and_byte(iso_content);

        let end_line_count = start_line_count + iso_line;
        let end_char_count = if iso_line > 0 {
            iso_char
        } else {
            start_char_count + iso_char
        };

        if position_in_range(
            (start_line_count, start_char_count),
            (end_line_count, end_char_count),
            target_line_char,
        ) {
            return index.wrap_some();
        }

        last_iteration_end_line_count = end_line_count;
        last_iteration_end_char_count = end_char_count;
        max_prev_span_end = iso_literal_end_index;
    }

    None
}
```

`position_in_range` and `line_and_byte` are unchanged.

```rust
// from crates/isograph_compiler/src/lib.rs
pub use iso_literals::{
    LineChar, LiteralId, iso_literal_extraction, iso_literal_text_at_location,
    literal_id_at_location, parsed_iso_literal, parsed_iso_literal_at_location,
};
```

Change 2 drops `iso_literal_text_at_location` and `parsed_iso_literal_at_location` from this list. Until then they still compile: they call `literal_id_at_location` then `iso_literal_extraction`.

```rust
// from crates/isograph_compiler/src/iso_literals.rs
#[memo]
pub fn iso_literal_text_at_location<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: PathBuf,
    line_char: LineChar,
) -> Option<String> {
    let literal_id = literal_id_at_location(db, path, line_char)
        .as_ref()?
        .clone();
    iso_literal_extraction(db, literal_id)
        .as_ref()?
        .iso_literal_text
        .clone()
        .wrap_some()
}

#[memo]
pub fn parsed_iso_literal_at_location<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: PathBuf,
    line_char: LineChar,
) -> Option<ParsedIsoLiteral> {
    let text = iso_literal_text_at_location(db, path, line_char)
        .as_ref()?
        .clone();
    parsed_iso_literal(db, text).clone().wrap_some()
}
```

Those two still clone a `String` or a tree per `LineChar`. Change 2 deletes them. They exist in this change only so existing tests keep compiling.

### Tests

Same `memo_tests` module. Intern with `intern_file`. One-line fixtures: `line` is 0, `character` is the byte index.

- No `DiskFile`: `literal_id_at_location` is `None`. `iso_literal_extraction` of `LiteralId { path, index: 0 }` is `None`.
- Intern `iso(\`entrypoint Query.HomeRoute\`)`. `character` is `contents.find("entrypoint")`. `literal_id_at_location` is `Some(LiteralId { path, index: 0 })`. `iso_literal_extraction` of that id matches `TypeScriptHostLanguage::extract_iso_literals` `[0]` (text, start index, context). `character` 0 (`i` of `iso`) is `None`. The `LineChar` of the last byte of the interior is `index: 0`. One past that last byte (the closing backtick) is `None`.
- Intern two literals on one line: `iso(\`field Pet.fullName { id }\`) iso(\`entrypoint Query.HomeRoute\`)`. A `character` inside the second interior is `index: 1`. A `character` of `) iso` is `None`.
- Intern `iso(\`\nentrypoint Query.HomeRoute\n\`)`. `{ line: 1, character: 0 }` is `index: 0`. `{ line: 0, character: 0 }` is `None`. `{ line: 2, character: 0 }` is `None`.
- Intern one literal. `iso_literal_extraction` of `LiteralId { path, index: 1 }` is `None`.
- Prefix the one-literal file with `const x = 1;\n`. The new interior `LineChar` is `{ line: 1, character: same byte-on-line as before }`. `literal_id_at_location` there is `LiteralId { path, index: 0 }`. The pre-prefix `LineChar` is `None`.
- Intern `const x = 1;\niso(\`entrypoint Query.HomeRoute\`)`. `LineChar { line: 1, character: 5 }`. Second `Present` replaces the first line with `const x = 1; const y = 2;` (still one newline). That `LineChar` is still `LiteralId { path, index: 0 }`.

Existing `iso_literal_text_at_location` / `parsed_iso_literal_at_location` tests stay until Change 2.

`expect` names the fixture the test interned.

## Change 2: delete text and tree at `LineChar`

Delete `iso_literal_text_at_location` and `parsed_iso_literal_at_location`. Delete the comment on the tree clone. Nothing stores `ParsedIsoLiteral` or the literal `String` at `(path, LineChar)`.

A caller that has `path` and `LineChar` and needs the tree:

```rust
let literal_id = literal_id_at_location(db, path, line_char)
    .as_ref()?
    .clone();
let extraction = iso_literal_extraction(db, literal_id).as_ref()?;
parsed_iso_literal(db, extraction.iso_literal_text.clone())
```

That is not a memo. Hover (later) is a memo on `(path, LineChar)` whose body does this and then the hover-specific work. The hover result can differ at every `LineChar`. The parse tree does not.

```rust
// from crates/isograph_compiler/src/lib.rs
pub use iso_literals::{
    LineChar, LiteralId, iso_literal_extraction, literal_id_at_location, parsed_iso_literal,
};
```

### Tests

Rewrite the `memo_tests` that called `iso_literal_text_at_location` or `parsed_iso_literal_at_location`.

`one_literal` (host-error tests) becomes:

```rust
// from crates/isograph_extract_typescript/src/lib.rs
let character = contents
    .find(extraction_text)
    .expect("the fixture contains the iso text") as u32;
let literal_id = literal_id_at_location(
    &db,
    path.clone(),
    LineChar {
        line: 0,
        character,
    },
)
.as_ref()
.expect("the interior is inside the literal")
.clone();
let extraction = iso_literal_extraction(&db, literal_id)
    .as_ref()
    .expect("the LiteralId indexes this file")
    .clone();
let parsed = parsed_iso_literal(&db, extraction.iso_literal_text.clone()).clone();
```

`extraction_text` is the interior, from `TypeScriptHostLanguage::extract_iso_literals` `[0].iso_literal_text` after intern, or `contents.find` of the known interior. Same as today: intern, then the first extraction's text.

Replace:

- No `DiskFile`: `literal_id_at_location` is `None`. `file_literals` is `None`. Drop `parsed_iso_literal_at_location`.
- Intern `iso(\`entrypoint Query.HomeRoute\`)`. Interior `LineChar` is `LiteralId` index 0. `iso_literal_extraction` of that id has `iso_literal_text` equal to that interior. `parsed_iso_literal` of that text has empty parse errors and `IsoLiteralItem::Entrypoint`. `character` 0 is `None` for `literal_id_at_location`. Last interior byte is index 0. One past is `None`.
- Intern `iso(\`entrypoint\`)`. `parsed_iso_literal` of the extraction text has parse errors non-empty.
- Intern `iso(\`\nentrypoint Query.HomeRoute\n\`)`. `{ line: 1, character: 0 }` is index 0. `{ line: 0, character: 0 }` and `{ line: 2, character: 0 }` are `None`.
- Two literals. Second interior is index 1. `parsed_iso_literal` of that extraction's text is `Entrypoint`. Between the literals, `literal_id_at_location` is `None`.
- Same literal text in two files: `parsed_iso_literal` of that text matches both files' extraction texts' parses.
- Prefix with `const x = 1;\n`. New interior `LineChar` is index 0. `iso_literal_extraction` of that id has the same `iso_literal_text` as before the prefix. `parsed_iso_literal` of that string matches. The pre-prefix `LineChar` is `None`.
- Lengthen the earlier line. Same `LineChar` is still index 0. `iso_literal_extraction` of that id has the same `iso_literal_text`. `parsed_iso_literal` of that string matches.

Host-error tests keep the same assertions. They go through the rewritten `one_literal`.

Do not add a production function only the tests call.

## Call sites

- Tests in this file: intern a `DiskFile`, `literal_id_at_location` at a `LineChar`, then `iso_literal_extraction` / `parsed_iso_literal`.
- Hover, goto, completion (later) -> `literal_id_at_location`, then extraction and parse of that id's text, then work that can differ per `LineChar`.
- file-semantic-tokens.md -> extract-all and `parsed_iso_literal`. Unchanged.
- `file_literals` -> extract-all and `parsed_iso_literal`. Unchanged.
- event-model.md item 7 and 8 still name `iso_literal_extraction(path, LineChar)` and `parsed_iso_literal_at_location`. That file updates when this lands.
