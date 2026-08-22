# Notes for later

- topological sort of variable defaults that reference other variables, e.g. `$foo = "foo", $bar = { foo: $foo }`, is a later stage (`reachable_variables` as `HashSet`)
