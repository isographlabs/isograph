# Compiler one-pager

A description of the compiler and how we are rewriting it from being a batch compiler to being an incremental compiler.

This document will be updated with a talk I gave about this, when it is made public.

## Overview

The compiler is written in Rust.

The compiler does several things:

- reads and parses the GraphQL schema,
- reads and parses isograph literals,
- validates them,
- creates the content of the generated files (called artifacts), and
- writes these artifacts to disk.

The compiler runs in batch mode and watch mode, and also powers the language server.

## Incremental rewrite

We are rewriting the compiler from being a batch compiler to being an incremental compiler. There are two main goals:

- re-use previously calculated values as much as possible, and
- allow the language server to provide meaningful results, even if the code is in an invalid state.

The first goal is self-explanatory. I will expand on the second. When editing code, it is often in an invalid state. For example, you may have a typo in one selection (`pictur`). Simultaneously, you may hover on another field (`fullName`). Validating `pictur` is not required to provide a meaningful hover result, so the language server should be able to!

If the compiler is a batch compiler, it will encounter a validation error, and fail to proceed. So, we won't be have some data structure where we can look up the description of the `fullName` field, and thus will not be able to provide meaningful hover results.

### Strategy

We created a memoization framework, [pico](https://github.com/isographlabs/isograph/commit/0d2745d09298b94fd4cd704965d461a05d66aea1). This framework is undergoing some changes, so some of this may differ from what is eventually settled on.

It exposes a derive macro `#[memo]` (temporarily called `#[legacy_memo]` while we change a few details). When applied to a function, it will cache the results of calling that function. The next time around, if that function is called again with the same arguments, and where none of the inputs it read have changed, it will reuse the cached value. An input is either a source (like the state of a given file) or another memoized function.

### Challenge one: parameters

Slapping `#[memo]` on every existing function provides remarkably little benefit. Consider a `get_validated_schema` function that takes every isograph literal AST as an input and returns a data structure describing every type and its available fields.

If we memoize this, we are creating a hashmap with a vector of all of the ASTs are a key. This will use up an unreasonable amount of memory, and calling `get_validated_schema` a second time will require comparing massive objects, which isn't performant.

So, instead, we must "invert" the compiler. Instead of passing the ASTs to `get_validated_schema`, we must have `get_validated_schema` call `get_asts`. Now, it is no longer a parameter, and our memory usage declines.

This is not an easy task, and is ongoing.

### Challenge two: many inputs

This `get_validated_schema` function depends on every AST, so any change to any isograph literal will invalidate this function. The compiler spends a majority of its time executing this function + everything that comes afterward, like writing files to disk, so we have a hard cap on how much pico can actually save us!

The answer here is to avoid having "chokepoints", which is to say, having massive data structures with many inputs that other things depend on.

Consider the function to generate some hover text for the user. If that function depends on `get_validated_schema`, then unrelated change will cause us to have to redo a lot of work to calculate the hover text. Inefficient!

The solution is to avoid having large data structures, and instead, encode the results as memoized functions. Consider hovering on `fullName`. That might involve checking:

- does a type named `Pet` exist? Is it an object?
- does it have a field named `fullName`?
- what is its description?

Notably, at no point do we check that `pictur` exists.

This is also not an easy task, and is ongoing.

## Other awesome Rust things

There are lots of really cool tricks we do:

- Every string is interened and has a newtype. This makes it hard to accidentally use the wrong string, which is a perpetual risk when working with compilers.
- We use proc macros extensively, for example, to generate methods to create a typesafe data structure to expose where the users' mouse is hovering. (You might imagine that the result is something like `{ item: fullNameScalarField, parent: { item: petObjectField, parent: { item: SomeClientField, parent: () }}}`, with typesafety throughout, i.e. a `ClientField` can only have a parent of `()`, and a scalar/object field can have either an object field or a client field as a parent, but not a scalar field).
- We treat "directives" as a serde format. In other words, we parse directives (`@loadable(lazyLoadArtifact: true)`) into something resembling JSON, then use that as an input to derive structs like `LoadableDirectiveSet` in a completely typesafe way. As a result, changes to `LoadableDirectiveSet` automatically result in changes to what we validly parse.
