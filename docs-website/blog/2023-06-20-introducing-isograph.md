---
slug: introducing-isograph
title: Introducing Isograph
authors: robertbalicki
tags: []
---

Please check out this [Substack article](https://isograph.substack.com/p/introducing-isograph) announcing Isograph! This article covers the intended developer experience of Isograph, and future features, such as:

- magic mutation fields (though not called such in the article),
- deferred resolvers,
- entrypoints, and
- injected analytics code.

It also makes the case that Isograph will be well-suited for apps that prioritize correctness, because:

- type of every field is very informative (e.g. the type of a field might indicate whether the field was unfetched, errored, null or present), and
- precise control over how to handle each state (e.g. to suspend when reading the resolver if the field is unfetched).

Thank you!
