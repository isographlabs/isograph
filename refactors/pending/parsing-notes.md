# Notes for later

- parse_iso_literal checks length before calling parse_singleton, maybe the latter can accept a non empty vec or something
- the distinction between argument and constant argument is wrong, it simply needs to allow for a topological sort, e.g. $foo = "foo", $bar = { foo: $foo } should be supported
