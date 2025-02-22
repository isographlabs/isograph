# Introducing pico

> Pico is our framework for on-demand, incremental computation. It is strongly influenced and inspired by [Salsa](https://github.com/salsa-rs/salsa), which is a "generic framework for on-demand, incrementalized computation" used by Rust Analyzer. Pico is a smaller, simpler salsa.
>
> It is usable independently of Isograph.

## Overview: incremental computation

### What role will incremental computation and Pico play within Isograph?

We aim to use Pico to improve the performance of the Isograph compiler and (future) language server. Furthermore, Pico should enable you to get these performance benefits without major changes to the structure of your codebase.

### Why is incremental computation important for compiler and language server performance?

Incremental computation enables the compiler to more quickly to small changes in input, for example when being run in watch mode. For example, you might imagine that the compiler will execute the following:

```
typecheck_project -> parse_ast("fileA") -> file_text("fileA")
                  -> parse_ast("fileB") -> file_text("fileB")
```

If the user changes only `fileA`, then it is unnecessary for the compiler to recompute `parse_ast("fileB")`, if it can instead reuse the value from the previous compilation.

Furthermore, a language server is even more suited to reuse of intermediate values, as subsequent requests will frequently make use of values that may have been previously computed. Consider a user who is hovering on various parts of an Isograph literal. The language server might execute:

```
hover("fileA", { row: 1, column: 1 }) -> parse_ast("fileA") -> file_text("fileA")
hover("fileA", { row: 1, column: 2 }) -> parse_ast("fileA") -> file_text("fileA")
```

As long as the underlying `file_text("fileA")` does not change, reusing `parse_ast("fileA")` can allow the language server to more quickly respond to the hover requests.

### What are the downsides to incremental recomputation?

- DevEx: plain functions may be simpler to write.
- Unenforced assumptions: memoized functions are assumed to be pure functions. This is not enforced.
  - In particular, one cannot rely on a memoized function actually being executed when it is invoked, instead of a value from a previous invocation being reused.
  - In addition, if a function is not pure, you may unexpectedly get wrong (stale) results.
- Additional memory usage: intermediate values need to be stored somewhere, and there is the added question of garbage collection and memory deallocation.
- Pico may contain bugs.

Please weigh these potential drawbacks before deciding to adopt Pico!

## Pico, from a user's perspective

There are three fundamental user-facing building blocks in Pico: the database, sources and memoized functions.

### The database

The database stores sources and cached values, and is responsible for efficiently executing memoized functions. You can often create it and forget about it.

There are two main ways to interact with the database:

- getting, setting and removing sources, and
- calling memoized functions

In addition, the database supports garbage collection. (See below.)

### Sources

Sources are the ground truth in your application. You are responsible for getting, setting and removing them. An example source might be the underlying file text. In a compiler running in watch mode, whenever a file change event is detected, the compiler will inform the database that the file text has changed by calling `db.set`.

An arbitrary struct with named fields can be a source, as long as it implements `Clone`, `PartialEq`, `Eq` and `pico_macros::Source`. It must have a field annotated with `#[key]`, which serves (along with the struct's type) as a unique identifier of a source of that type.

This is an example of setting and then updating a source in the database:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

let mut db = Database::default();

let input_id: SourceId<Input> = db.set(Input {
    key: "framework",
    value: "pico".to_string(),
});

let input_id_2 = db.set(Input {
    // Because this `key` is identical, we are updating the source in the database, and
    // input_id_2 == input_id
    key: "framework",
    value: "isograph".to_string(),
});
```

Setting an item in the database will return a `SourceId<T>` (annotated above for clarity; you do not need to do this in practice.) Hold onto these and pass them to memoized functions!

### Memoized functions

Top-level functions can be annotated with `#[pico_macros::memo]`. They must take a first parameter with type `&Database`, as well as up to 8 additional parameters. (This restriction will be lifted at some point!)

When this function is executed, it will check if we have previous invoked this function, and whether the value is still cached (i.e. has not been garbage collected).

- if a previous value is available, check whether this value can be reused.
  - if so, reuse the value from the previous invocation
  - if not, re-invoke the function.
- if no value is available, invoke this function.

> "Whether the value from the previous invocation can be reused" is a complex topic. See the implementation section below for more details.

A memoized function is assumed to be a pure function of its parameters, of any inputs it reads and of any other memoized functions it calls. It should also have no side effects.

An example use of memoized functions might look like:

```rs
fn test() {
    let mut db = Database::default();

    let input_id: SourceId<Input> = db.set(Input {
        key: "framework",
        value: "pico".to_string(),
    });

    assert_eq!(*capitalized_first_letter(&db), 'P');

    // When we re-invoke the function, it will not actually be called. Instead, we will
    // reuse the cached value 'P'.
    assert_eq!(*capitalized_first_letter(&db), 'P');
}

#[memo]
fn first_letter(db: &Database, input_id: SourceId<Input>) -> char {
    let input = db.get(input_id);
    input.value.chars().next().unwrap()
}

#[memo]
fn capitalized_first_letter(db: &Database, input_id: SourceId<Input>) -> char {
    let first = first_letter(db, input_id);
    first.to_ascii_uppercase()
}
```

In this example, the memoized function `capitalized_first_letter` calls `first_letter`, which reads `input_id` from the database. Re-invoking the function, when no source has changed, will cause pico to reuse the cached result (`'P'`).

## Pico implementation

> There's a lot more! This is a short summary. Stay tuned for a longer blog post with plenty of examples on [isograph.dev](https://isograph.dev).

### The fundamental algorithm

Pico keeps track of a global `epoch`, which is simply a counter that increases whenever a source is updated.

For each source, it keeps track of the epoch at which it was updated.

For each memoized function call (really, for a function + roughly the hash of its parameters), it keeps track of the epoch at which it was verified (i.e. the current epoch at which it was invoked) and a back-dated epoch since which its value could not have changed ("time updated").

For example, if the current epoch is 2, and a source `S` was updated at epoch 1, and a memoized function `f` that reads `S` is invoked for the first time, it will end up with a time verified of 2 and a time updated of 1.

When that function is re-invoked, we check whether the value could have possibly changed (A):

- Is the current epoch equal to the time verified? If so, we know that nothing could have changed and we can immediately reuse the value.
- If not, we iterate over each dependency.
  - For each source dependency, we ask "has this source changed since the memoized function's time verified"? If any source dependency has so changed, we recalculate the memoized function.
  - For each derived dependency (i.e. if a memoized function calls another memoized function), we ask whether that memoized function could have possibly changed (A).

If any dependency has changed, we re-invoke the function and compare the results. If the values are identical, we say that this function has not changed.

In addition, we update the memoized function's verified at to be the current epoch.

This allows to efficiently recompute just values that could have changed. This is very abstract and hard to understand! Let's make this more concrete with an example.

### An example

#### Initial setup and first invocation

Consider a chain of memoized functions: `typecheck -> parse_ast -> file_text`, where the first two are derived functions, and `file_text` is a source.

We set `file_text` and invoke `typecheck` for the first time. Now, the state of the database is as follows:

```
current_epoch: 1
file_text | updated_at: 1
parse_ast | updated_at: 1, verified_at: 1
typecheck | updated_at: 1, verified_at: 1
```

#### Re-invocation with no changes to sources

Next, we re-invoke `typecheck`. We ask, can we reuse the cached value? First, we check whether it was verified during the current epoch. It has been! Great. We reuse the cached value. Efficient!

#### Re-invocation after a relevant change to a source

Next, we update `file_text`, causing the `current_epoch` and its `updated_at` to be `2`.

The state of the database is now:

```
current_epoch: 2
file_text | updated_at: 2
parse_ast | updated_at: 1, verified_at: 1
typecheck | updated_at: 1, verified_at: 1
```

Now, we re-invoke `typecheck`. We ask, can we reuse the cached value? First, we check whether it was verified during the current epoch. It hasn't been, so we iterate over each dependency and see whether that dependency has changed since `typecheck` was last verified(epoch 1). The only dependency is `parse_ast`. (A)

So, we ask, can we reuse the cached value for `parse_ast`? First, we check whether it was verified during the current epoch. It hasn't been, so we iterate over each dependency and see whether it has changed. The only dependency is `file_text`.

We check whether `file_text` has changed since `parse_ast` was verified. It has!

So, we know we much re-execute `parse_ast`. We re-execute it, find that the result is different than the previously cached result, store the new value (B), and mark it as having been updated and verified during the current epoch (2).

So, now we continue from (A). We know we must re-invoke `typecheck`, since a dependency has changed since epoch 1. So, we re-execute it, store the new value, and mark it as having been verified and updated during the current epoch (2).

Note that this time, when we actually execute `typecheck`, we call `parse_ast`. And it was verified during the current epoch, so we simply reuse the value we stored in (B)!

Whew! After all of that, the state of the database is:

```
current_epoch: 2
file_text | updated_at: 2
parse_ast | updated_at: 2, verified_at: 2
typecheck | updated_at: 2, verified_at: 2
```

#### Re-invocation after a relevant change to a source with short circuiting

Okay, so now let's say we update `file_text` (current epoch: 3, updated at: 3), but in a way that doesn't affect the AST. Perhaps we just added a comment or manipulated some spacing.

Again, we re-invoke `typecheck`. We ask, can we reuse the cached value? First, we check whether it was verified during the current epoch. It hasn't been, so we iterate over each dependency and see whether it has changed since `typecheck` was last verified (epoch 2). The only dependency is `parse_ast`. (A)

So, we ask, can we reuse the cached value for `parse_ast`? First, we check whether it was verified during the current epoch. It hasn't been, so we iterate over each dependency and see whether it has changed. The only dependency is `file_text`.

We check whether `file_text` has changed since `parse_ast` was verified. It has!

So, we know we much re-execute `parse_ast`. We re-execute it, find that the value has not changed, and mark it as having been verified during the current epoch. We indicate that the value has not changed since epoch 2!

Now, continuing from (A), we find that no dependency has changed since epoch 2! So we are able to reuse the previous value we got from calling `typecheck`. Efficiency!

The state of the database is now

```
current_epoch: 3
file_text | updated_at: 3
parse_ast | updated_at: 2, verified_at: 3
typecheck | updated_at: 2, verified_at: 3
```

Note that for `parse_ast` and `typecheck`, `updated_at` is 2, while `verified_at` is 3. In this step, we've determined that the result of these memoized function calls could not have changed since 2, so we are **backdating**.

#### Why is backdating important?

Backdating allows us to short-circuit!

Let's assume that, before the previous step, we also had invoked another function `print_ast`, which depends on `parse_ast`, and it was last verified in epoch 2.

Everything described in the previous step still occurred, i.e. we modified `file_text` and re-invoked `typecheck`.

So the state of our database is:

```
current_epoch: 3
file_text | updated_at: 3
parse_ast | updated_at: 2, verified_at: 3
typecheck | updated_at: 2, verified_at: 3
print_ast | updated_at: 2, verified_at: 2
```

Now, we re-invoke `print_ast`. We check each dependency, and ask whether it could have possibly changed since `print_ast` was last verified (epoch 2). In other words, we are checking whether the dependencies have changed epoch 2.

If we hadn't backdated, it would look like `parse_ast` could have changed in epoch 3. But since we backdated, we know that `parse_ast` did not change between epochs 2 and 3 (which is the current epoch), so we can reuse the cached value for `print_ast`!

### More resources

[This](https://youtu.be/i_IhACacPRY?si=LAKfTJUxOgFNiwmI) is an amazing video going into this algorithm. The [Salsa book](https://salsa-rs.github.io/salsa/) is also an amazing resource.

## Garbage collection

The Database will accumulate cached values indefinitely, until you call `run_garbage_collection`.

Calling `run_garbage_collection` will:

- collect the last N top-level memoized function calls (defaults to `10_000`) in an LRU cache.
- traverse from those top-level calls, and retain all reachable intermediate values (and params, but that isn't really user-facing). Drop (i.e. garbage collect) the remainder.

In addition, you can mark certain top-level calls as retained by calling `db.retain`. This will give you a guard that, while active, will prevent that top-level memoized function call, and anything reachable from it, from being garbage collected.

## Credit

Thank you to @edmondop for finding usability issues in Pico when incorporating it into the Isograph compiler!
