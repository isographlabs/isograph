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
fn syntax_highlighting(parent_db_view: &mut DatabaseView, file_name: FileName) -> &SyntaxHighlighting {
  parent_db_view.calculate<T>(
    "syntax_highlighting",
    file_name: T,
    // inner function:
    |self_db_view, file_name: &FileName /* &T */| {
      let asts = get_asts(self_db_view, file_name); // tracked
      calculate_syntax_highlighting_from_asts(asts) // pure
    }
  )
}

// Or if we have a proc macro that does the db_view.calculate stuff:
#[track]
fn syntax_highlighting(db_view: &mut DatabaseView, file_name: &FileName) -> SyntaxHighlighting {
  let asts = get_asts(nested_db_view, file_name);
  calculate_syntax_highlighting_from_asts(asts) // this is a SyntaxHighlighting
}
```

Here, `db_view.calculate` is called, which creates a derived node for the tuple `("syntax_highlighting", file_name)`.

#### What does `calculate` do when called initially?

If `db.calculate` is called for a given `(name, param)` tuple, and no derived node for the `(name, param)` tuple exists in the Database, it will:

- call the inner function and store its value in the Database at `(name, param)`, while
- tracking its dependencies,
  - i.e. track whenever `DatabaseView::calculate` is called within that inner function, e.g. via `get_asts_in(nested_db_view, ...)`.
- set the derived node's `time_calculated` to the maximum `time_calculated`/`time_set` epoch of the dependencies (`e0`), (this can be `u32::MAX` if there are no dependencies),
  - Note: this is because the value would have been the same if this fn was called as early as `e0`
- and set the derived node's `time_verified` to the max of the current epoch `e1` and of `time_calculated` (which can be `u32::MAX`).

In the derived node in the Database for `("syntax_highlighting", file_name)`, it will track that `get_asts_in` was called (which is tracked because it called `DatabaseView::calculate`).

The resulting value (a `SyntaxHighlighting`) is stored in the database, and a reference to it is returned to the caller.

#### What about when `calculate` is called again?

If `calculate` is called again (at epoch `e2`), we "verify" the node. Assume that the node has `time_calculated = e0` and `time_verified = e1` (notably, `e1 !== e2`).

We verify each dependency, which returns `time_calculated/time_set` for each dependency.

If the dependencies all have `time_calculated/time_set <= e1`, then, we return a reference to the last calculated value.

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
- causes AST to be calculated. That derived node isn't in the Database, so we call its inner function, which checks the source text (`time_set = e0`)
- AST has `time_verified = e1` and `time_set = e0`
- hover info is calculated, which also has `time_verified = e1` and `time_set = e0`.

### Source text is updated and syntax highlighting is called

- source text is set, `time_set = e2`
- syntax highlighting is calculated. The derived node isn't in the Database, so we call its inner function, which
- causes AST to be calculated. The derived node _is_ in the Database, and `e2 > ast.time_verified` (`e1`), so we check its dependencies. They have max `time_set = e2`, which is greater than `e1` (AST's `time_verified`), so we recalculate AST by calling its inner function. AST now has `time_verified = e2` and `time_set = e2`.
- syntax highlighting is calculated, which also has `time_verified` `e2` and `time_set` `e2`

### No updates, but go to definition is called

- go to definition is calculated. The derived node isn't in the Database, so we call its inner function.
- This calls `calculate_ast(db_view)`. The `ast` derived node _is_ in the Database, and `ast.time_verified === current_epoch`, so we return the current value of `ast`.
- go to definition's inner function is completed, so that has `time_verified = e2` and `time_calculated = e2`.

## Implementation details

### `Database`

- Stores the global epoch (an incrementing counter)
- Has a `derived_nodes: HashMap<(&'static str, HashString)> -> DerivedNode`
  - `HashString` is the params hashed
- Has `source_nodes`? But these maybe could be separate

### `DerivedNode`

- `result: Box<dyn Any>`
- perhaps `params: Box<dyn Any>` (if we need to recover them)
- the `time_verified` epoch
- the `time_calculated` epoch.
  - Invariant: `time_calculated <= time_verified`
- perhaps: `time_accessed` epoch (for garbage collection)
- `Vec<Dependency>`

### Dependencies

A sketch of how to store dependencies

```rs
struct DatabaseView<'parent, 'db: 'parent> {
  database: &'db Database,
  dependencies: Vec<Box<dyn Fn() -> bool>> + 'parent
  parent_view: &'parent DatabaseView
}

impl<'parent, 'db: 'parent> DatabaseView<'parent, 'db> {
  fn calculate<TParam: Any, TOutput: Any>(
    &'parent mut self,
    static_key: &'static str,
    param: TParam,
    inner_fn: impl Fn(&'db mut DatabaseView /* self db view */, &TParam) -> TOutput + 'parent;
  ) -> &'db TOutput {
    let parent_db_view = self;
    // tell parent_db_view that "syntax_highlighting" was called
    parent_db_view.dependencies.push(Box::new(DerivedNodeDependency {
      static_key,
      param_hash: hash(param),
      recalculate_inner_no_params: Box::new(|database_view| {
        inner_fn(database_view, param)
      }),
    }));

    if let Some(mut previously_derived_node) = self.database.get_mut(static_key, param) {
      let has_changed = false;
      for revalidate_dependency in previously_derived_node.dependencies.iter_mut() {
        has_changed = revalidate_dependency();
        if has_changed {
          break;
        }
      }
      if has_changed {
        let nested_view = self.nested_view();
        let derived_node_value = Box::new(inner_fn(&mut nested_view, &param));

        parent_db_view.max_time_calculated = std::cmp::max(
          parent_db_view.max_time_calculated,
          nested_view.max_time_calculated,
        );

        // database.store returns a reference to the derived_node_value
        return self.database.store(static_key, param, DerivedNode {
          derived_node_value, // Box<dyn Any>,
          time_verified: self.database.epoch,
          time_calculated: nested_view.max_time_calculated,
          dependencies: nested_view.dependencies,
        }).downcast_ref::<TOutput>().expect("Expected to be the correct type")
      } else {
        return &previously_derived_node.derived_node_value;
      }
    } else {
      panic!("same thing as above");
    }

  }

  fn nested_view(&mut self) -> DatabaseView {
    Self {
      database: self.database,
      dependencies: vec![],
      max_time_calculated: 0,
    }
  }
}

impl Database {
  fn revalidate<'db>(
    &'db mut self,
    parent_static_key: &'static str,
    parent_param: HashString,
    parent_recalculate_inner: impl Fn(&'db mut DatabaseView) -> Box<dyn Any + Eq> + 'db
  ) -> ChangeStatus {
    match self.get_mut(static_key, param) {
      Some(previously_derived_node) => {
        // check each dependency
        // if all dependencies are not invalidated, return HasNotChanged
        // if any dependency is invalidated, recalculate_inner, compare, and
        // if different, store and return HasChanged
        // always set revalidated at to current epoch

        let dependency_change_status = ChangeStatus::Unchanged;
        for dependency in previously_derived_node.dependencies.iter_mut() {
          if let Some(dependency_pdn) = self.get_mut(dependency.static_key, dependency.param_hash) {

              if (dependency_pdn.time_validated === self.current_epoch) {
                continue;
              } else {
                // potentially
                dependency_change_status = ChangeStatus::Changed;
                break;
              }

          } else {
            panic!("Expected nested dep to exist");
          }
        }
        match dependency_change_status {
          ChangeStatus::Unchanged => {
            return ChangeStatus::Unchanged
          },
          ChangeStatus::Changed => {
            let mut database_view = self.database_view();
            let new_value = parent_recalculate_inner(&mut database_view);

            previously_derived_node.dependencies = database_view.dependencies;
            previously_derived_node.time_verified = self.current_epoch;

            if new_value != previously_derived_node.derived_node_value {
              previously_derived_node.derived_node_value = new_value;
              previously_derived_node.time_calculated = database_view.max_time_calculated;
              return ChangeStatus::Changed
            }

            return ChangeStatus::Unchanged
          },
        }
      }
      None => {
        panic!("Expected to exist if revalidating")
      }
    }
  }
}

enum ChangeStatus {
  Changed,
  Unchanged,
}


```

### `DatabaseView`

## Notes

- Perhaps we can find a way to have typed storage, instead of `Box<dyn Any>` everywhere.
- Perhaps we should intern all the results.
- How would garbage collection occur? It seems safe to drop items from the `derived_nodes` map, since they will simply be recalculated. (It is not safe to drop from `derived_node_inner_functions`)

## To discuss

- references
- hash maps
- tracked inputs

# unneeded

- `derived_node_inner_functions: HashMap<&'static str, Box<dyn Fn(&mut Database, Param) -> Box<dyn Any>>>`
  - This looks difficult to statically type.
  - Why is this struct necessary?
    - e.g. when checking `hover_info`, we need to check all of its dependencies (without calling `hover_info.inner_function`!) This check could find that `AST` is stale (i.e. has dependencies that are updated). At this point, we need to call `AST`'s inner function. At this point, we compare `ast.new_value == ast.old_value`, and _if these are the same_, we can short circuit and avoid calling `hover_info.inner_function`.
    - Thus, we need the ability to call `ast.inner_function` without going through `hover_info.inner_function`.
