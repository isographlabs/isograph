---
title: GraphQL Conf 2023
authors: robertbalicki
tags: []
---

Please check out this [presentation about Isograph](https://www.youtube.com/watch?v=gO65JJRqjuc) at GraphQL Conf 2023. Please also see [the slides](https://docs.google.com/presentation/d/1ffot9Dmy2Z5YFnr6hEjAlr_Fn4Nrzc-zEnFK54QA6Jo/edit#slide=id.g27f8644aff7_0_1).

<!-- truncate -->

In this presentation, Robert Balicki covers:

## Introduction and motivation

- The many steps that one goes through when one extracts a child component
- Data masking, and why it is important
- The fundamental tension at the heart of GraphQL: it is a language for querying servers being used to provide isolation to front-end components
- The big idea: **every fragment is associated with exactly one function**

## Isograph

- The developer experience that Isograph aims to provide
- The other big idea: **resolvers are GraphQL's missing primitive**
- How in Isograph, **everything is a resolver**
- How in Isograph, even components can be resolvers
- How entrypoints work in Isograph

## Why resolvers are the unit of abstraction

- If we achieve data masking, we're already using resolvers
- Deferring data and code together is natural
- Resolver components let us reason about the on-screen-ness of data
- Imperatively loaded resolvers are entrypoints
- Server executed resolvers are regular ol' GraphQL resolvers
- Conditionally loaded resolvers are `@match` and Relay 3D
- Resolvers can lead to better DevTools and application-creation tools

## A demonstration of Isograph: Robert's Pet List 3000™

## The roadmap

## Q&A

- Is it worth writing your own Isograph language, instead of reusing GraphQL?
- What is the migration path from Relay to Isograph? Can it be automated?
- How does Isograph handle client resolver and server field name collisions?
- Is Isograph a bundler?
- How tied is Isograph to React, GraphQL and JavaScript?

I hope you check it out!
