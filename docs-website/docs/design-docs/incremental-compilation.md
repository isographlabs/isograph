# Incremental compilation

:::note
This document outlines how incremental computation might be done in the Isograph compiler. It won't necessarily be exactly how it is implemented.
:::

## Why

In watch mode and when running the compiler as a language server, we want to efficiently recompute values in response to changes in input. When an input changes, we could recompile everything from scratch, but it would be much more efficient to recompute the minimal section of the computation graph that actually needs to change.

Consider this computation graph, which we might encounter if we have both file1 and file2 open simultaneously:

```
syntax highlighting (file1) -> iso literal ASTs in (file1) -> file text (file1)
syntax highlighting (file2) -> iso literal ASTs in (file2) -> file text (file2)
```

If the text of file1 changes, we don't need to recompute syntax highlighting (file2), and can just reuse the last computed value.

But even more subtly, if the file 1 text changes, but in a way that doesn't affect the AST (e.g. the user adds some JavaScript after the last iso literal), then we can recompute the "iso literal ASTs in (file1)" node, determine that the result didn't change, and thus avoid recomputing the "syntax highlighting (file1)" node.

But how can that be implemented?

## Types of nodes

There are two types of nodes in the above graph: source nodes and computed nodes.

### Source nodes

The "file text" nodes are the only source nodes shown above. A source node is the "raw input". A source node contains:

- its value, and
- `time_set`, the "epoch" when it was last set
  - note: the epoch is a monotonically increasing `u32`, stored in the Database

#### What happens when a source node changes?

When a source node is updated, it's value is compared to the old one. If different, we increment the Database's epoch, update the stored value and set `time_modified` to the current epoch.

### Computed nodes

A computed node is a deterministic function that always receives, as its first parameter, the database (as `&mut self`), and as its second parameter, receives a single struct of parameters. It must always call `db.calculate` immediately. This is not enforced, but could be via `#[derive]` macro.

```rs
impl Database {
  fn syntax_highlighting(&mut self, file_name: FileName) -> &SyntaxHighlighting {
    db.calculate("syntax_highlighting", file_name, |nested_db, file_name: &FileName| {
      let asts = nested_db.get_asts_in(file_name);
      calculate_syntax_highlighting_from_asts(asts) // this is a SyntaxHighlighting
    })
  }
}
```

Here, `db.calculate` is called, which creates a derived node for the tuple `("syntax_highlighting", file_name)`.

#### What does `calculate` do when called initially?

If `db.calculate` is called for a given `(name, param)` tuple, and no derived node for the `(name, param)` tuple exists in the Database, it will:

- create a derived node in the Database
- call the inner function and store its value in the Database at `(name, param)`, while
- tracking its dependencies,
  - i.e. track whenever `db.calculate` is called within that `db.calculate`, e.g. via `nested_db.get_asts_in`.
- set the derived node's `time_calculated` to the maximum `time_calculated`/`time_set` epoch of the dependencies (`e0`),
  - Note: this is because the value would have been the same if this fn was called as early as `e0`
- and set the derived node's `time_verified` to the current epoch `e1`.

In `syntax_highlighting`, it will track that `nested_db.get_asts_in` is called (which itself calls `db.calculate`).

The resulting value (a `SyntaxHighlighting`) is stored in the database, and a reference to it is returned to the caller.

#### What about when `calculate` is called again?

If `calculate` is called again (at epoch `e2`), we "verify" the node. Assume that the node has `time_calculated = e0` and `time_verified = e1` (notably, `e1 !== e2`).

We verify each dependency, which returns `time_calculated/time_set` for each dependency.

If the dependencies all have `time_calculated/time_set < e1`, then, we return a reference to the last calculated value.

If not, then we call the inner function, and compare the new result to the old result. If `new_result != old_result`, we update the stored value and set `time_calculated` to the max `time_calculated/time_set` epoch of the dependencies.

In either case, we set `time_verified` to `e2`.

#### What if we verify a node and `time_verified === current_epoch`?

This may occur if no changes to source files have occured! So, we can just return the same value. This might occur, for example, if we want hover information for a node and syntax highlighting. Both can use the same AST.

## Example

Consider this dependency tree:

```
hover info   syntax highlighting   go to definition
          \ /                      /
          AST --------------------/
           |
      source text
```

### Hover info is called

- source text is set, `time_set = e0`
- several other changes occur, epoch is now `e1`.
- hover info is calculated. The derived node isn't in the Database, so we call its inner function, which
- causes AST to be calculated. That derived node isn't in the Database, so we call its inner function, which checks the source text
- AST has `time_verified` `e1` and `time_set` `e0`
- hover info is calculated, which also has `time_verified` `e1` and `time_set` `e0`.

### Source text is updated and syntax highlighting is called

- source text is set, `time_set = e2`
- syntax highlighting is calculated. The derived node isn't in the Database, so we call its inner function, which
- causes AST to be calculated. The derived node _is_ in the Database, and `e2 > ast.time_verified` (`e1`), so we check its dependencies. They have max `time_set = e2`, which is greater than `e1` (AST's `time_verified`), so we get the value of `source text` and recalculate AST. AST now has `time_verified = e2` and `time_set = e2`.
- syntax highlighting is calculated, which also has `time_verified` `e2` and `time_set` `e2`

### No updates, but go to definition is called

- go to definition is calculated. The derived node isn't in the Database, so we call its inner function.
- This causes AST to be calculated. The derived node _is_ in the Database, and `ast.time_verified === current_epoch`, so we return the current value of `ast`.
- go to definition's call is completed, which has `time_verified = e2` and `time_calculated = e2`.

## Implementation details

### `Database`

- Stores the global epoch (an incrementing counter)
- Has a `derived_nodes: HashMap<(&'static str, HashString)> -> DerivedNode`
  - `HashString` is the params hashed
- `derived_node_inner_functions: HashMap<&'static str, Box<dyn Fn(&mut Database, Param) -> Box<dyn Any>>>`
  - This looks difficult to statically type.
  - Why is this struct necessary?
    - e.g. when checking `hover_info`, we need to check all of its dependencies (without calling `hover_info.inner_function`!) This check could find that `AST` is stale (i.e. has dependencies that are updated). At this point, we need to call `AST`'s inner function. At this point, we compare `ast.new_value == ast.old_value`, and _if these are the same_, we can short circuit and avoid calling `hover_info.inner_function`.
    - Thus, we need the ability to call `ast.inner_function` without going through `hover_info.inner_function`.
- Has `source_nodes`?

### `DerivedNode`

- a `Box<dyn Any>` for the outcome
- perhaps `Box<dyn Any>` for the params (if we need to recover them)
- the `time_verified` epoch
- the `time_calculated` epoch.
  - Invariant: `time_calculated <= time_verified`
- perhaps: `time_accessed` epoch (for garbage collection)

## Notes

- Perhaps we can find a way to have typed storage, instead of `Box<dyn Any>` everywhere.
- Perhaps we should intern all the results.
- How would garbage collection occur? It seems safe to drop items from the `derived_nodes` map, since they will simply be recalculated. (It is not safe to drop from `derived_node_inner_functions`)
