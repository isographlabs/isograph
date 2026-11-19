# i2

From upstream isograph we keep pico and pico_macros (unchanged), the swc plugin and its dependency chain (isograph_config, common_lang_types, string_key_newtype, prelude), resolve_position, the relay crates, the docs website, and the build process. The new work is `crates/isograph_parser`, `crates/graphql_schema_parser` (SDL parse, unit tests in that crate; `graphql_network_protocol` is the only crate that depends on it, and parses schema files with it), `crates/tests`, `crates/graphql_network_protocol` (the GraphQL `NetworkProtocol`; core crates do not depend on it), `crates/isograph_cli` (freddie_cli's lifecycle verbs around the daemon), and `crates/ts_graphql_react_isograph_cli` (the `isograph` binary).

The parser does not use pico. Parser functions are plain functions over `&str`. Memoization applies above the parser (which files changed, which literals were extracted). The iso parser is tokenize, match brackets, chunk, grammar consume. The GraphQL schema parser is tokenize, then grammar consume. We do not add tree-sitter, a tree-sitter grammar, or a tree-sitter crate.

## Upstream isograph

Before writing a type, function, error, or pass, check the isograph checkout (`../isograph`) for the existing pattern. If isograph has a good one, use it.

For every design decision, first state what upstream isograph does (`path:line` in `../isograph`). The held option is upstream's behavior unless there is a stated reason to deviate. A deviation names that reason.

## Mental model

Schema data model: `docs-website/docs/design-docs/mental-model.md`. Read it before working on entities, entity declarations, wrappers, selectables, selectable declarations, selections, selection sets, or entrypoints.

pico: `docs-website/docs/design-docs/pico.md`. Read it before working on sources, memos, or `IsographDb`. Read its "Structuring functions and memos" section before writing a memo or an LSP handler.

## pico tracked maps

`#[tracked]` on a database field (`disk_file_map`) gives `get_disk_file_map()` / `get_disk_file_map_mut()`. `handle` writes the map with `tracked()`; `MutView` has no `untracked()`.

A memo that iterates the map (compile, every file) uses `tracked()`. That records a dependency on the whole map. Inserting or removing any key re-invokes it.

A one-key lookup does not read the map. It is `db.try_get(T::id_for_key(&key))`, which returns `Option<&T>` and records a dependency on that key whether or not it is set. A later set or remove of that key re-invokes the reader. A set or remove of another key does not. `file_contents` reads `EditorBuffer::id_for_key(&EditorBufferKey { client, path })`, then `DiskFile::id_for_key(&path)`.

Do not iterate `untracked()`. A newly inserted key is not seen. pico `tracking_field::correctness` pins this.

## Commits

Commit after every change, small and atomically, without being asked. Each logical change is its own commit.

`git commit` runs `.pre-commit-config.yaml` in the dedicated worktree `<clone>-precommit` (for this clone, `/Users/rbalicki/code/i2-precommit`). The invoking working tree, index, and stash list are not modified. The `cargo clippy` and `cargo test` hooks run on the workspace crates the commit changes plus their reverse dependencies (`scripts/affected-crates.sh`), and on the full workspace when the commit changes `Cargo.toml`, `Cargo.lock`, `clippy.toml`, or `.cargo/`. CI runs both on the full workspace. `git commit --no-verify` skips the hooks. This clone needs `git config core.hooksPath "$(git rev-parse --show-toplevel)/githooks"` run from the `i2` worktree so the stored path is absolute and every worktree of this clone runs that hook.

## Memory

No Claude memory files. Disregard recalled memories. Project rules live here, in `docs-website/docs/design-docs/mental-model.md`, and in `refactors/`.

## Quiescent codebase

A quiescent codebase is one in which, over the last 5 minutes, there are no changes to files other than planning docs: no commits and no working-dir changes.

## Refactor docs

Plan in `refactors/`. Docs for a specific project can be grouped: `refactors/pending/project/foo.md` and `refactors/past/project/foo.md`. Move a doc from `refactors/pending` to the same relative path under `refactors/past` when we will not work on it again. Do not modify `refactors/past/`.

An actively worked doc:

- Describes what we are building. Not how we got there, what we are not building, thought process, or what has already landed.
- Has every big change already discussed with the user (in the doc or in chat), especially inner/core/reusable code. A surprise is a reason not to proceed.
- Is enough for a new agent to implement with no important decisions left. Write out every type in full (fields and variants, not a prose description), every function signature, who calls it, what it returns, and the end-user experience (what the user does and sees). No stubs. Before/after snippets for every change, including new types and functions.
- Writes generated or expanded code only when changing the generator, or when the expansion is a novel shape. A derive that matches an established pattern needs only the attribute change. Generated blocks start with `// generated by $file`.
- Copies extracted code verbatim, names the origin, and states the delta. An unstated difference is a defect.
- Opens every snippet with `// from $file`. Generated blocks use `// generated by $file` instead. Code copied from another repo uses `// external $repo/$file`, the other repo's name then the path inside that repo (`freddie/crates/freddie_cli/src/client.rs`, `figaro/figaro_model/src/model/mod.rs`). Pending-doc postfix lint skips a rust snippet whose first non-empty line starts with `// external`.
- Orders types most important first, not define-before-use.
- Prefers code snippets over paragraphs.
- Follows the coding standards below.

No scratch pad in this repo. No tests for work still under discussion.

A doc may split into a problem discussion and an ordered list of independently shippable changes, prefactors first. Identify those and ship them as prefactors. If a refactor is too large, say so and split into conceptually different files.

Stale is fine while a doc is idle. Update it when we start working on it. If we are working on `A` and `B` depends on `A`, do not keep `B` current unless `B` is in the discussion.

Stay in the doc until the user gives explicit permission to implement. "Looks good," edits to the doc, "go on," "continue," and further planning are not permission. Ambiguity means do not implement.

If a previous requirement becomes unachievable, raise it with the user.

## Implementing a refactor doc

Stop and ask as soon as the doc stops matching the code. Fix the doc first.

Stop on unexpected complexity: a redesign, a new shared-state primitive, a new trait, or a crate the doc never mentioned. Say what exploded and the options; the user picks.

`git stash` half-finished work if that leaves a cleaner tree. Say what you stashed and its state.

Do not paper over a gap by choosing the easy version, leaving a TODO, or narrowing the step so it fits.

## Workarounds

If a task cannot be fulfilled without a workaround, ugly code, or a violation of an invariant or coding standard, and the user has not explicitly approved that:

1. Attempt to avoid the workaround. Think carefully about how to make the solution align with the standards and architecture.
2. If that does not work, raise it with the user.

The final architecture is always clean. There is never a reason to introduce unclean or janky things. If the architecture cannot achieve the goal, the architecture changes until it can, with elegance. Shipping janky code is never an acceptable step.

Things to look out for include mixing levels of logic in a function. If a function passes a parameter onto another function and does not inspect it, inspecting that parameter and handling one specific case is a red flag.

## Tests

No snapshot tests; they pin implementation, not behavior. Assert facts about the result. Test behavior, not implementation.

Fixtures are inputs only. Expected results are assertions in the test, never checked-in expected-output files.

`expect` is allowed with a reason that names an invariant the test itself established (a fixture it built, an env the harness sets). Production code is not a test fixture.

An API only tests call should not exist. A helper tests need lives in the tests crate or under `#[cfg(test)]`.

Do not test another crate. pico, intern, serde, std. Re-invoke counts, intern identity, and `ptr::eq` of interned values are pico or intern. Assert `Eq` of the values those memos return.

## Wake on events

No polling. No sleeping. An idle process is parked, not waking every N milliseconds to look. Work arrives by waking whatever the consumer is blocked on: the channel it `recv`s, the socket it `read`s, the flock wait, the OS watcher. A loop that retries a predicate until a deadline fires is polling, including `sleep` then `try_recv`. Capture the position first (subscribe, open the follow) then produce the event, so a wake cannot be lost if it races the wait. `select!` or a woken channel is the shape.

A regular file does not become readable through `epoll`/`kqueue` when it grows. Watch the file (the directory, for create) with the OS watcher, then read. Do not `sleep` at EOF.

Absence has no edge of its own. Inject a later event that will fire, wait for that, and assert the forbidden thing did not arrive first.

## Event loop

`run_event_loop` recvs an event, calls `handle`, and sends the returned effects. It does not grow more logic. It does not match on the event. It does not match on the effect. It does not keep a table, filter, or other interpretation of what arrived or of what `handle` returned. That work is `handle` (a field on state) or `perform` (the effect).

## Shared state and interior mutability

`Arc`, `Rc`, `Mutex`, `RwLock`, `Cell`, `RefCell`, `OnceCell`/`OnceLock`, `lazy_static`, `thread_local!`, and atomics are almost always the wrong reach. `handle` is a pure function of `IsographState` and event on one thread; the state lives in one place, and a caller that wants a value already holds it. A primitive that shares or mutates through a lock is papering over a design that put the value in the wrong place. Question whether it is needed, default to the version that does not need it, and raise it with the user every time before it goes into a planning doc or into code. Name the primitive, what it is sharing, and why the non-shared version does not work.

The preferred way to move data between threads is a channel whose sender is freely `Send` and cloneable while the receiver stays pinned to one thread. Sending an event to the worker that owns `IsographState` beats reaching into that state across a lock. If a design reaches for `Arc<Mutex<_>>`, the first question is what channel would carry that data instead.

Ambient state the program itself owns belongs on `IsographState` or a local on the task that owns it, not in a `static` or a `thread_local!`. A process-wide counter, a next-id source, anything `handle` reads or advances: make it a field and thread it to where it is used, so the value it hands out is a function of state. A `static NEXT: AtomicU64` is the shape to avoid: it is ambient, it makes the `handle` that reads it impure, and it is usually `Sync` only to satisfy `static` rather than because two threads touch it.

This is distinct from state the outside world owns. Disk files and editor buffers arrive as events (`DiskChanged`, `EditorChanged`); they are external truth mirrored into the database, not ambient state the program invented. The test is who mints the value. If the program mints it, it is a field on state. If the OS or the editor owns it, it arrives as an event.

## Booleans

Prefer an enum whose variants name the states. A `bool` field, parameter, or return type: question whether it is needed, default to the enum, and raise it with the user every time before it goes into a planning doc or into code. Name the `bool` and the two cases. Fall back to `bool` only when the value is a pure yes/no with no domain names.

A field that exists only when a flag is set is `Option<T>` or an enum with a payload-bearing variant, not `flag: bool` plus `payload: Option<T>`. Two booleans that cannot both be true are one enum.

## Panic and UnexpectedError

A broken programmer invariant the type system cannot express is `LSPRuntimeError::UnexpectedError` or a panic (`panic!`, `assert!`, `expect`, `unwrap`, `unreachable!`, `todo!`, `unimplemented!`). `UnexpectedError` surfaces to the user (an LSP error response). Usually `UnexpectedError`. Panic is the other choice. Do not pick panic because you did not look up `UnexpectedError`.

An `Option` or `Result` the body will always unwrap should not have been optional. A match arm that is always unreachable is a state the function should never have been handed. An `Infallible` error type (including `Result<T, Infallible>`) is a `Result` that should not be a `Result`. Question whether it is needed, default to types that do not need it, and raise a panic with the user every time before it goes into a planning doc or into code. Name the construct, the invariant, and why the type system cannot express it.

The parser never panics on any input. Malformed input is a representable state. Parses of user-supplied text, file reads, and socket reads return a typed error or a recovered region.

## Coding standards

Make impossible states unrepresentable. Use the correct representation. Extra refactoring to get the right building blocks is required. Prefer the structurally correct solution to the easy one, even when the easy one is fast. Reuse the seams the code already has. Over-engineering is good when it constrains future agents that lack this context.

We do not have anything we do not use. A token kind, field, variant, or recorded value exists only because something reads it. Tests-only is not a reader. A semantic token, span, or resolve leaf is a reader only if it carries information worth highlighting or hovering. An AST field is a reader only if resolve or a later pass uses it.

A token that carries no semantic information is not recorded and is not a resolve leaf. Consume it if the parse needs the presence, and leave it off the child's span. A position on it resolves to the parent. `!` after a type decides Null vs not and is this case.

A more performant, less idiomatic impl lives behind a newtype/struct/enum that exposes an idiomatic API.

No comment that restates the code, that describes what wasn't done, or that describes what lives elsewhere unless a reader of this file has a concrete reason to expect it here. A doc comment names what the thing is and stops.

Never rely on discipline what we can enforce with newtypes.

If std or an existing crate provides it, use that. A hand-rolled version needs a stated reason in the planning doc that introduces it.

Avoid custom traits. Prefer a concrete type, an enum, a plain function, or a std trait (`From`/`Into`, `Default`, the iterator traits). A trait with one implementor is the case to avoid. When a design introduces a trait, say what it buys over a concrete type, and default to the version without it.

Always test degenerate cases.

## Never throw information away

A cardinal rule. Never discard a distinction the program has already established. The type of a value must be as narrow as what is still possible.

Matching some variants and binding the rest at the original type throws information away:

```
Foo(Bar::A) => handle_a(),
Foo(another_bar) => handle_remainder(another_bar),
```

`another_bar` is `Bar`. That type still includes `A`. The match already ruled `A` out. Never do this. The remainder's type must not include `A`. Introduce that narrower type, or match every remaining variant.

If a function does not return every variant of an enum, return something narrower. Same rule. Propagating a final error type is allowed; it is still an anti-pattern.

Calling a function that takes `Option<T>` when you have a `T` (`foo(t.wrap_some())`) is the same defect on the input side. The callee still admits `None`. That is generally an anti-pattern, especially when `foo` contains business logic. If we own `foo`, rewrite it to take `T`, and add a `foo_opt` that takes `Option<T>` and calls `foo` internally.

## Result

Do not `match` a `Result` to bind the success value and return or convert the error. Write `let value = expr.map_err(...)?;`. Same type on both arms: `expr.unwrap_or_else(|err| ...)`.

Do not pass a value used only on one arm of the `Result` or `Option` the function returns, when the caller has that same arm. Return the success side; the caller fills the other. `require_token(kind)` plus `map_err(|()| cursor.expected(expected))`, not `require_token(kind, expected)`. Same for `require_group`.

## `&mut T`

Return values; do not thread mutable state. If a function could take `&mut Vec<T>` and only pushes, do not pass the vec. Return `T`, or pass `impl FnOnce(T)` (or `FnMut`) that itself pushes. The caller writes `|t| vec.push(t)` next to the vec.

## Iterators

Prefer iterator combinators over `for`, `while`, and `loop`. Do not `.collect()` into a `Vec` unless a later step needs the owned list (random access, stored length, ownership past the walk, or an API that takes a collection). A once-consumed walk stays an iterator.

A loop is for a state machine the iterator traits do not express: a parser cursor, a worklist, a walk whose next step depends on mutation that is not the iterator's item. Pushing to a vec, setting a flag, or returning the first match is not that case.

## Errors

Error types use `thiserror`: `#[error("...")]` on the derive, `#[error("{0}")]` when wrapping. No manual `Display` or `std::error::Error` impl.

A non-error type must not implement `std::error::Error`. A non-error enum that needs `Display` uses `strum::Display` (`#[strum(to_string = "...")]` on the variants).

`From` for an enum variant's payload is `derive_more::From`. `#[from]` on the variants that should convert; omit it or `#[from(skip)]` on the others. Do not hand-write `impl From<T> for TheEnum`. strum is `Display` / `FromStr` / `FromRepr`, not `From<payload>`.

## Postfix wrappers

Do not write prefix or constructor wrappers that `prelude::Postfix` or `span::WithSpanPostfix` already names.

- `Ok(value)` / `Err(value)` / `Some(value)` → `value.wrap_ok()` / `value.wrap_err()` / `value.wrap_some()`
- `Box::new(value)` → `value.boxed()`
- `vec![value]` (one element) → `value.wrap_vec()`
- `value.into()` → `value.to()`
- `*value` when copying out of a `Deref` whose target is `Copy` → `value.dereference()`
- `&value` when taking a shared reference to a value → `value.reference()`
- `WithSpan::new(item, span)` → `item.with_span(span)`

Patterns stay (`if let Some(x)`, `match r { Ok(v) =>`, `let Err(e) =`, `matches!(x, Some(_))`). `None` stays `None`. `&T` / `&str` / `&[T]` in type position, `&mut`, `&self`, `&mut self` stay. Empty `vec![]` and `vec![a, b, ..]` stay. Prefix `&` also stays when `Postfix` cannot express it: `s[i..]` is unsized, `let x = &foo()` needs temporary lifetime extension, `&|...|` for higher-ranked `Fn` bounds. `.to()` takes a turbofish when the target type is not inferred.

The std forms appear only in the method bodies in prelude. `WithSpan::new` appears only in `span::WithSpanPostfix::with_span`. `clippy.toml` in `crates/prelude` bans `dbg` / `dbg_with_note` / `note_do_not_commit` and flags `Ok`/`Err`/`Some`, `Box::new`, one-element `vec![]`, and `.into()`. It does not flag `.reference()` / `.dereference()`; still write them in new code.

## Audits

An audit fixes the whole class, not the quoted instance. Sweep every file the standard touches before reporting done.

## Coding standards: nits

- Enums are unit variants (`NoData`) or a single named payload (`Named(NamedTypeAnnotation)`, `List(Box<ListTypeAnnotation>)`). `Foo { x: T }` is a named struct, then `Foo(TheStruct)`. `Foo(A, B)` is `Foo(FooPair)` or `Foo((A, B))`.
- Effect types are `FooEffect` (`IsographEffect`, `SendLspResponseEffect`). Event types are `FooEvent` (`IsographEvent`, `LspEvent`, `InternalEvent`, `ClientConnectionEvent`, `ClientStartedEvent`). The enum variant that holds one does not include `Effect` or `Event`: `IsographEffect::SendLspResponse(Box<SendLspResponseEffect>)`, `IsographEvent::Lsp(LspEvent)`, `IsographEvent::Internal(InternalEvent)`, `IsographEvent::ClientConnection(ClientConnectionEvent)`, `ClientConnectionEvent::Started(ClientStartedEvent)`.
- Span-carrying is decided once, at the type: every use of an enum is `WithSpan`-wrapped or none is, and within one enum every variant's payload carries its span or none does. A value's span lives on its nearest wrapper, exactly once: the field wrapper when the value is a struct field, the item wrapper when the value rides in an enum whose items are wrapped.
- A value that does not exist in the source has no span. Do not invent an empty span, a sibling's span, or the parent's span to fill `WithSpan`. A collection of source and synthetic nodes is `WithOptionalSpan<T>` (`WithGenericLocation<T, Option<Span>>`). `Some(span)` is the source range. `None` is not in the text. `ResolvePosition` does not enter `None`. `()` as a location is a tree with no positions at all (spanless parsing), not a missing node in a spanned tree.
- A struct with exactly one field is a newtype (`struct Foo(pub Bar)`), except shapes an external derive dictates (serde field names, clap flag names). If another field is suspected, keep the named-field struct.
- Parser interned-key wrappers are `$RoleWrapper` over the `common_lang_types` interned type (`ArgumentNameWrapper(ArgumentName)`, `SelectionNameWrapper(SelectionName)`, `SelectableNameWrapper(SelectableName)`). Inner field is `pub`; resolve returns the wrapper; callers read `.0`. No `From<StringKey>`: `string_key_newtype!` already does that on the inner type. Construction is `token.interned().map(SelectionNameWrapper)`. A selection name and a `reader_alias` are `SelectionNameWrapper`. An entrypoint name and a selectable name are `SelectableNameWrapper`. The left-hand side of `Type.name` is `EntityNameWrapper`. `Description` is the same shape (`pub DescriptionValue`) without the `Wrapper` suffix.
- Selection, Selectable, and SelectionSet are defined in `docs-website/docs/design-docs/mental-model.md`. Do not name a selection node `Selectable*`. Checking that a selection refers to a selectable that exists is a later pass.
- Prefer `map.entry(k).or_insert(...)`, `or_default`, `and_modify`, or a match on `Entry` over `contains_key` / `get` / `get_mut` plus `insert` when both reading and writing a slot.
- Try not to mix match levels in one function. If each variant calls a function, do not inline other arms. If each arm matches one level, do not match two levels on some arms. `Result<Option<T>, E>` and `Option<Result<T, E>>` break this regularly and correctly. Guidance, not a hard rule.
- Never `#[allow(lint)]`. Use `#[expect(lint)]`. An item whose only callers are tests is `#[cfg_attr(not(test), expect(dead_code))]`. Clippy `allow_attributes` is deny in workspace lints and covers outer `#[allow]` only; do not use inner `#![allow]` in our crates either. Relay crates keep crate-level `#![allow(clippy::all)]`.

## TypeScript

These rules cover hand-written TypeScript: the runtime, the Babel plugin, and the VS Code extension. Generated artifacts are plain object literals and types.

- Private state is an ES private field (`#field`), not TypeScript's `private`.
- A method that reads `this.#x` throws a `TypeError` when called through a `Proxy` wrapping the instance, because the `Proxy` does not have the private field. Do not wrap an object that has private fields in a `Proxy`.
- Properties are `readonly` and arrays are `ReadonlyArray<T>` by default.
- No `any`. Use `unknown` and narrow.
- Target ES2022 or later. ES2022 compiles private fields and class fields natively. An older target turns each private field into a `WeakMap` lookup.

## Invariants - do not work around these

- Never throw information away. A remainder after matching some variants is a narrower type that does not include those variants, not the original type.
- No tree-sitter. Parsers are tokenize and grammar consume in our crates (the iso parser also match-brackets and chunks). A highlighting or schema-parse need is that pipeline, not a grammar.js.
- No manual `ResolvePosition` impls. If one seems needed, that is a missing feature in `resolve_position`; add it as a prefactor first.
- `run_event_loop` is recv, `handle`, send those effects. It does not match on the event or the effect. New behavior is an event `handle` interprets, a field on state `handle` writes, or an effect `perform` runs.
