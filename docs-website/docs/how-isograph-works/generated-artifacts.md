# Generated artifacts

:::warning
The precise nature of the generated artifacts is liable to change.
:::

The Isograph compiler generates artifacts in the `artifact_directory` folder. These fall into three types:

- reader artifacts
- entrypoint artifacts
- refetch artifacts

## Reader artifacts

The reader artifact is generated at `TypeName/field_name/reader.isograph.ts`.

A reader artifact contains an import of the resolver function (i.e. the `Query/home_page_component/reader.isograph.ts` will contain an import of `` export const home_page_component = iso` Query.home_page { ... }`  ``) and the reader AST. It will als contain some types that ensure that whatever data is passed to the resolver function is accessed in a typesafe fashion.

The reader AST is a data structure that is used to read out precisely the fields and resolvers that that resolver function selected.

## Entrypoint artifacts

The entrypoint artifact is generated at `TypeName/field_name/entrypoint.isograph.ts`.

An entrypoint (e.g. `iso entrypoint Query.home_page_component `) is always associated with a single resolver. The entrypoint artifact contains:

- the query text
- the normalization AST
- a hard require of the reader artifact

It should also contain the type of the variables, but does not.

Entrypoints are used to make network requests and write the data back to the Isograph store.

## Refetch artifacts

Refetch artifacts are generated at `TypeName/field_name/__refetch__${NUMBER}.isograph.ts`. They are used for `__refetch`'s **and** for magic mutation fields.

Refetch artifacts can be thought of as entrypoints for a sub-section of a query. They contain:

- the query text
- the normalization AST

They are not associated with a specific resolver, and so do not have a reader artifact.

### Why are they numbered?

Refetch artifacts are numbered, because they can be used by multiple resolvers. Consider:

```
User.profile_component {
    id,
    name,
    avatar_component,
    __refetch,
}

User.avatar_component {
    avatarUrl,
    __refetch,
}
```

In this case, the refetch query will refetch the `{ id, avatarUrl, name }`. The refetch artifact that is used when `__refetch` is called on the `User.profile_component` and `User.avatar_component` resolvers is the **same** resolver.

Thus, since they are re-used by resolvers and not clearly tied to a specific resolver, they are numbered.

Note that the refetch artifact used by a resolver is not always the same one, either. Consider:

```
User.profile_component {
    user_detail_component,
    best_friend {
        user_detail_component,
    }
}

User.user_detail_component {
    id,
    name,
    __refetch,
}
```

In this case, the `user_detail_component`'s refetch artifact will select `{ id, name, best_friend { id, name }}`. The `best_friend.user_detail_component`'s refetch artifact will select `{ id, name }`. So, where a resolver is selected affects what fields are selected in refetch and magic mutation field queries.
