# Four trees

Tokenize, match brackets, chunk, and grammar are four stages. The bracket tree and the chunk tree nest the same groups. Leftover grammar clones content items into `UnparsedChunkItems`.

They are not one tree with a different item type. `MatchedBrackets` is a sequence of raw tokens and `Bracketed` groups. `ChunkedLevel` is a sequence of `Chunk`s: a separator-free run plus an optional trailing separator. Chunking introduces that run. `OpenBracket` / `CloseBracket` are `Copy`; copying them into `ChunkedGroup` is not the cost.

The owned clone that matters is leftover `UnparsedChunkItems` (unread remainder, and today the whole chunk on failure). leftover-in-extra.md is that clone's contents and highlighting. Do not parse during chunking. Do not borrow `Slot.extra` from the chunk tree (that puts a lifetime on the grammar tree).

No code change in this doc. leftover-in-extra.md is the work.
