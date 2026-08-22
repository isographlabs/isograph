# Return a reference to an item of a list intern

The list intern is everything in the file: `THostLanguage::extract_iso_literals(path)` is `Option<Vec<IsoLiteralExtraction>>`. The item intern is one element of that vec, keyed by `LiteralId` (the file plus the 0-based extract index). pico must store a reference to `list[index]`, compare that item with `==` on re-invoke, and return `&T`. The item memo does not clone the element. Concat, diagnostics, and cursor APIs all go through the list; an item is `list[index]`, not a second copy of the same extraction.

Origin of the list: `extract_iso_literals`. Origin of the item key: `LiteralId`. Origin of pointing at a list element without cloning: pico `intern_ref_chain` (`get_economists` then `intern_ref` of an element). Delta: the item memo is already keyed by list plus index, so the stored value is a reference into that slot, not a `MemoRef` whose identity is a hash of the item. Callers see `&IsoLiteralExtraction` the same way they see `&Vec` from the list. `Hash` is not required on the item.

One shippable change: pico accepts a memo body that returns a borrow of an item it read from another memo (or source), and `iso_literal_extraction` uses it.

## What the user does

No editor-facing change. Tests intern a file with one iso literal, call extract and `iso_literal_extraction` of `LiteralId { path, index: 0 }`, and assert the item Eq-equals the first element of the extract vec. A prepend keeps index 0 and moves `iso_literal_start_index`. An append leaves the item `==`.

## Types

```text
THostLanguage::extract_iso_literals(path)
  -> Option<Vec<IsoLiteralExtraction>>

iso_literal_extraction(LiteralId)
  -> extract(path)[index]   // reference, not clone
```

```rust
// from crates/isograph_compiler/src/iso_literals.rs
#[memo]
pub fn iso_literal_extraction<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    literal_id: LiteralId,
) -> Option<IsoLiteralExtraction<THostLanguage>> {
    let extractions = THostLanguage::extract_iso_literals(db, literal_id.path.clone()).as_ref()?;
    extractions.get(literal_id.index)
}
```

Written return type stays `Option<IsoLiteralExtraction<THostLanguage>>`. pico lookup stays `&Option<IsoLiteralExtraction<THostLanguage>>`. The body returns `Option<&IsoLiteralExtraction<_>>` (`Vec::get`). pico stores that as a reference into the extract vec. It does not `.cloned()`.

`None` is no `DiskFile`, or `index` past the extract vec.

Re-invoke compares the pointed-to `IsoLiteralExtraction` with `==`. Prepend: extract `!=` (start index moved), the item `!=`, dependents of the item re-invoke. Append: extract `==`, the item memo does not re-invoke. `parsed_iso_literal` of the item's text is unchanged in both cases.

A memo that returns a field of a struct it read (`&str` into a `DiskFile`, `&String` into an extraction) is the same feature. This change's first caller is a vec index.

pico `intern_ref` stays for "find in the list, identity is the value's bits" (`get_economist_by_name`). This change is "identity is already the index."

## Tests

Compiler `memo_tests`:

- No `DiskFile`: `iso_literal_extraction` is `None`.
- Present file, no iso: extract is `Some` of empty vec. `LiteralId { path, index: 0 }` is `None`.
- One literal. `iso_literal_extraction` of index 0 Eq-equals `extract[0]`.
- Two literals. Index 1 Eq-equals `extract[1]`. Index 2 is `None`.
- Prefix with `const x = 1;\n`. Index 0 Eq-equals the new `extract[0]`. `iso_literal_text` matches. `iso_literal_start_index` moved by the prefix length.
- Append `"\nconst y = 1;\n"`. Item `==` the pre-append extraction (same text, start index, context).

`expect` names the fixture the test interned.

## Call sites

- `iso_literal_extraction` as above. Existing callers that `.as_ref()?` keep working.
- Later file-level walks (concat, `file_literals`) index the extract vec for start index and text. They do not clone an extraction to get those fields.

Amend `docs-website/docs/design-docs/pico.md` when this lands: the item intern of a list is a reference into the list, keyed by index.
