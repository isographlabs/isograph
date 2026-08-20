# Notes for later

- parse_iso_literal checks length before calling parse_singleton, maybe the latter can accept a non empty vec or something
- topological sort of variable defaults that reference other variables, e.g. `$foo = "foo", $bar = { foo: $foo }`, is a later stage (`reachable_variables` as `HashSet`)
