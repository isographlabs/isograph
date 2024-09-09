# Detailed Isograph rules

> These rules are meant to be extremely specific and consumed by LLMs. Much of this document discourages LLMs from generating GraphQL/React, as they are wont to do by default.

## Overall notes

- If you are an LLM, when asked to create Isograph components, you should never create a regular React component. Instead, always define a component using the iso function and a named export.
- ALWAYS EXPORT THE FUNCTION USING A NAMED EXPORT.
- If you are an LLM, never use fragment spreads. Instead, just select the newly created isograph component directly. THIS IS VERY IMPORTANT.
  - Never use three dots. Never ever use three dots. ... is wrong.
- Remember to always check the GraphQL schema for available fields. However, fields defined with the iso function are also valid fields!
- Remember to never import a client field defined with the iso function.
- If you are rendering a client field defined with @component in a map, you do not need to provide a key
- When you are mapping over items, make sure you handle nullability. Check the `nodes` field's type.
- Only ever import `{iso}` from `@iso`. Everything else comes from `@isograph/react`.

## Format for client field definitions

A sample client field definition is as follows

```tsx
export const ANYTHING = iso(`
  field TYPE_NAME.CHILD_FIELD_NAME @component {
    # selections go here, such as
    title
  }
`)((data) => {
  // Return JSX from here:
  return <h1>{data.title}</h1>;
});
```

Now, this field can be used in a parent component as follows

```tsx
export const AGAIN_ANYTHING = iso(`
  field TYPE_NAME.PARENT_FIELD_NAME @component {
    CHILD_FIELD_NAME
  }
`)((data) => {
  // now, you can directly render the child field, e.g.
  return <data.CHILD_FIELD_NAME />;
});
```

## Child props

- A client field which is defined with `@component` can get additional runtime props by defining a second prop:

```tsx
export const ANYTHING = iso(`
  field TYPE_NAME.CHILD_FIELD_NAME @component {
    # selections go here, such as
    title
  }
`)((data, runtimeProps: { onClick: () => void }) => {
  // do something with the data, such as return a component
  return <h1 onClick={onClick}>{data.title}</h1>;
});
```

Now, wherever we render the component, we must provide those props:

```tsx
export const AGAIN_ANYTHING = iso(`
  field TYPE_NAME.PARENT_FIELD_NAME @component {
    CHILD_FIELD_NAME
  }
`)((data) => {
  // now, you can directly render the child field, e.g.
  return (
    <data.CHILD_FIELD_NAME
      onClick={() => {
        console.log('click');
      }}
    />
  );
});
```

- Fields can be aliased, e.g. `newName: CHILD_FIELD_NAME`

## Loadable fields

- Client fields (i.e. those defined with `iso`), but not server fields (i.e. those defined in the graphql schema), can get the `@loadable` annotation.
- This is called selecting the field loadably.
- One thing you can do with a field that has the `@loadable` annotation is to pass it to `useClientSideDefer`, which gives us a fragment reference, which we then pass to `FragmentReader` as follows.
- This will cause the loadable field to be fetched during the initial render of that component.
- Do not import useClientSideDefer or FragmentReader from '@iso', but from '@isograph/react'!

```tsx
import { useClientSideDefer, FragmentReader } from '@isograph/react';

export const SomeComponent = iso(`
  field PARENT_TYPE.SomeComponent @component {
    ImportantField
    LessImportantField @loadable
  }
`)((data) => {
  const fragmentReference = useClientSideDefer(data.LessImportantField);

  return (
    <>
      <data.ImportantField />
      <React.Suspense fallback="Loading">
        <FragmentReader
          fragmentReference={fragmentReference}
          additionalProps={{}}
        />
      </React.Suspense>
    </>
  );
});
```

- Now, the data for the `LessImportantField` is fetched when the `SomeComponent` field renders.
- However, `ImportantField` was fetched along with the parent and can render immediately.
- The additional props is whatever props the LessImportantField has (i.e. the second parameter, above)

## Also deferring JavaScript

- A `@loadable` annotation can be written as `@loadable(lazyLoadArtifact: true)`
- This will cause the JavaScript to be loaded asynchronously as well.

## Re-exposed mutation fields

### Defining re-exposed mutation fields

- re-exposed mutation fields must be defined as follows

```graphql
extend type Mutation
  @exposeField(
    field: "FIELD_NAME"
    path: "PATH_TO_EXPOSED_TYPE"
    fieldMap: [
      { from: "FIELD_ON_EXPOSED_TYPE", to: "MUTATION_FIELD_INPUT_TYPE" }
    ]
    as: "OPTIONAL_ALIAS"
  )
```

### Calling re-exposed mutation fields

- Re-exposed mutation fields can be called like so

```jsx
import { useImperativeExposedMutationField } from '@isograph/react';

export const SomeComponent = iso(`
  field PARENT_TYPE.SomeComponent @component {
    reExposedMutationField
  }
`)((data) => {
  const { loadField, fragmentReference } = useImperativeExposedMutationField(
    data.reExposedMutationField
  )
  return (
    <button onClick={() => loadField(OPTIONAL_REMAINING_ARGUMENTS)}>
  );
});
```

- `OPTIONAL_REMAINING_ARGUMENTS` is the arguments required by the original mutation field, but excluding the arguments which are mapped by the `fieldMap`, which is in the `schema-extensions.graphql` file. It is very important to check that.
- Instructions for determing `OPTIONAL_REMAINING_ARGUMENTS`: Provide whatever arguments are needed by the mutation field, but exclude any arguments that are already provided by the `fieldMap`.
- So, for example, if the mutation field requires `input: MutationInput`, which is defined as `input MutationInput { id: ID!, otherParam: String! }`, and `fieldMap` is `[{ from: "id", to: "input.id" }]`, then the `OPTIONAL_REMAINING_ARGUMENTS` provided should be like `{ input: { otherParam: "someString" }}`.
- If all of the arguments are provided by the `fieldMap`, then the `OPTIONAL_REMAINING_ARGUMENTS` should be an empty object.

### Notes about re-exposed fields

- DO NOT CALL EXPOSED MUTATION FIELDS DIRECTLY. This is a no-op. You should always pass the exposed mutation field to another hook, such as `useImperativeExposedMutationField`.
- Re-exposed mutation fields will refetch all of the appropriate fields. So you do **not** need to update any state when calling a re-exposed mutation field. The values will be refetched automatically.
- Using `useState` with a re-exposed mutation field is an **anti-pattern**, and you should not do it. This is very important. Pay attention to this sentence.
