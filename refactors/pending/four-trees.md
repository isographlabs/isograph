# Four trees

Tokenize, match brackets, chunk, and grammar are four stages. Each stage's tree is a different type. The bracket tree and the chunk tree nest the same groups. Leftover grammar clones content items into `UnparsedChunkItems`.

This doc has no code change. leftover-in-extra.md is the leftover clone's contents. leftover-semantic-tokens.md records `leftover_token` on extra and extra_chunks.

## The four stages

Tokenize takes `&str` and returns `Vec<WithSpan<IsographLangTokenKind>>`. It does not emit `EndOfFile`. Spaces are skipped. Line breaks are tokens.

Match brackets takes that vec and returns `MatchedBrackets`: a sequence of raw tokens and `Bracketed` groups.

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub struct MatchedBrackets(pub Vec<WithSpan<BracketItem>>);

pub enum BracketItem {
    Raw(NonBracketToken),
    Bracketed(Bracketed),
}

pub struct Bracketed {
    pub opening: WithSpan<OpenBracket>,
    pub children: WithSpan<MatchedBrackets>,
    pub closing: WithSpan<CloseBracket>,
}
```

Chunk takes the bracket tree and returns `ChunkedLevel`: a sequence of `Chunk`s. A chunk is a separator-free run of items plus an optional trailing separator. Chunking is the pass that introduces that run.

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct ChunkedLevel(pub Vec<WithSpan<Chunk>>);

pub struct Chunk {
    contents: NonEmpty<WithSpan<ChunkContentItem>>,
    trailing_separator: Option<WithSpan<ChunkSeparator>>,
}

pub enum ChunkContentItem {
    NonBracket(NonBracketToken),
    Group(ChunkedGroup),
}

pub struct ChunkedGroup {
    pub opening: WithSpan<OpenBracket>,
    pub children: WithSpan<ChunkedLevel>,
    pub closing: WithSpan<CloseBracket>,
}
```

`OpenBracket` / `CloseBracket` are `Copy`. Copying them into `ChunkedGroup` is not the cost that matters.

Grammar walks the chunk tree and builds `IsoLiteralParse`. Leftover that the grammar did not consume is owned `UnparsedChunkItems` on `Slot.extra`, and whole extra chunks on `Singleton.extra_chunks`. leftover-in-extra.md is that clone's contents (unread remainder, and a singleton trailing separator). Today a failed form clones the whole chunk; leftover-in-extra.md makes extra the unread remainder.

The owned clone that matters is leftover `UnparsedChunkItems`. Grammar does not parse during chunking. `Slot.extra` is owned, not a borrow of the chunk tree, so the grammar tree has no lifetime on the chunk tree.
