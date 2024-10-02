# Isograph

The framework for teams that move fast — without breaking things.

Isograph makes it easy to build robust, performant, data-driven apps.

- Read the [docs](https://isograph.dev/docs/introduction/), especially the [quickstart guide](https://isograph.dev/docs/quickstart/).
- Watch the [talk at GraphQL Conf 2024](https://www.youtube.com/watch?v=sf8ac2NtwPY) (and from [2023](https://www.youtube.com/watch?v=gO65JJRqjuc)).
- Join the [Discord](https://discord.gg/rjDDwvZR).
- [Follow the official Twitter account](https://twitter.com/isographlabs).

## What is Isograph?

Isograph is a UI framework for building React apps that are powered by GraphQL data. It has ambitions to be a framework for apps powered by data.

It has four goals:

- to remove as much friction as possible from the process of building data-driven apps
- to give developers the confidence that they won't break production
- to make it easy to build performant apps
- to expose powerful primitives so that developers can precisely model their domain

## About Isograph: Fetching data and app structure

Let's do a quick tour of how a basic Isograph app is constructed.

### What is Isograph, and what are client fields?

Isograph is a framework for building React applications that are backed by GraphQL data. In Isograph, components that read data can be selected from the graph, and automatically have the data they require passed in. Consider this example Avatar component:

```jsx
export const Avatar = iso(`
  field User.Avatar @component {
    avatar_url
  }
`)(function AvatarComponent(data) {
  return <CircleImage image={data.avatar_url} />;
});
```

This defines a new client field named `Avatar`, which is then available on any GraphQL User. You might use this avatar field in another component, such as a button that navigates to a given user's profile.

```jsx
export const UserProfileButton = iso(`
  field User.UserProfileButton @component {
    Avatar

    # you can also select server fields, like in regular GraphQL:
    id
    name
  }
`)(function UserProfileButtonComponent(data) {
  return (
    <Button onClick={() => navigateToUserProfile(data.id)}>
      {data.name}
      <data.Avatar />
    </Button>
  );
});
```

These calls to `iso` define client fields, which are functions from graph data (such as the user's name) to an arbitrary value. With Isograph, it's client fields all the way down — your entire app can be built in this way!

Note what we didn't do:

- The `Avatar` component didn't care how the `avatar_url` field was originally fetched. It just received it.
- When writing the `UserProfileButton` component, we didn't import the `Avatar` client field
- The `UserProfileButton` didn't pass any data down to the `Avatar`. It just rendered it! In fact, it didn't see or have access to any of the fields that the `Avatar` selected, so, changes to the fields that the `Avatar` selects will not change the behavior of other client fields!

### How does Isograph fetch data?

At the root of each page, you will define an entrypoint. Isograph's compiler finds and processes all the entrypoints in your codebase, and will generate the appropriate GraphQL query.

If the compiler encounters ``iso(`entrypoint Query.UserList`);``, it would generate a query that would fetch all the server fields needed for the `Query.UserList` client field and all of the nested client fields that are reachable from that root.

We might set up a component to fetch that `UserList` data as follows:

```jsx
function UserListPageRoute() {
  const queryVariables = {};
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Query.UserList`),
    queryVariables,
  );

  const additionalRenderProps = {};
  const Component = useResult(fragmentReference);
  return <Component {...additionalRenderProps} />;
}
```

> Note that the call to `useResult(fragmentReference)` will suspend if the required data is not present in the store, so make sure that either `UserListPageRoute` is wrapped in a `React.Suspense` boundary, or that the `fragmentReference` is only read in a child component that is wrapped in a suspense boundary.

Now, when `UserListPageRoute` is initially rendered, Isograph will make an API call.

### How do components receive their data?

You may have noticed that when we rendered `<data.Avatar />`, we did not explicitly pass the data that the `Avatar` needs! Instead, when the component is rendered, Isograph will read the data that the `Avatar` component needs, and pass it to `Avatar`. The calling component:

- only passes additional props that don't affect the query data, like `onClick`, and
- does **not** know what data `Avatar` expects, and never sees the data that `Avatar` reads out. This is called **data masking**, and it's a crucial reason that teams of multiple developers can move quickly when building apps with Isograph: because no component sees the data that another component selected, changing one component cannot affect another!

### Big picture

At the root of a page, you will define an entrypoint. For any such entrypoint, Isograph will:

- Recursively walk it's dependencies and create a single GraphQL query that fetches **all** of the data reachable from this root.
- When that page renders, or possibly sooner, Isograph will make the API call to fetch that data.
- Each resolver will independently read the data that it specifically required.

## About Isograph: `@loadable` fields

Selections of client fields can be declared as `@loadable`, meaning that the data for that client field is not included as part of the parent request. Instead, the value that is read out contains a function that you can call to make a new network request for just the `@loadable` field. Consider:

```jsx
export const UserDetailPage = iso(`
  field User.UserDetailPage {
    name
    CreditCardInfo @loadable
  }
`)((data) => {
  const CreditCardInfo = useClientSideDefer(data.CreditCardInfo);

  return (
    <>
      <h1>Hello {data.name}</h1>
      <React.Suspense fallback="Loading credit card info">
        <FragmentReader fragmentReference={CreditCardInfo} />
      </React.Suspense>
    </>
  );
});
```

In this example, the `CreditCardInfo` component is slow to calculate. This might be because it has to make an API call to an external service. We would not like to slow down the entire page as a result of that. So, instead, label this field `@loadable`.

Now, instead of returning a component that can be directly rendered, we get back something that contains a function that executes the network request to fetch the `CreditCardInfo`'s data. We pass that to `useClientSideDefer`, which makes that network request during the initial render of the component.

There we go! Now, our parent component can load quickly, and we make a follow-up request for the rest of the data!

:::note
You are not expected to use the `@loadable` field directly. Instead, always pass it to a handler like `useClientSideDefer`.
:::

## About Isograph: `@exposeField`

:::note
The `@exposeField` feature will change before the next release.
:::

> Currently, `@exposeField` is only processed if it is on the Mutation type. But, it will be made more generally available at some point.

Types with the `@exposeField(field: String!, path: String!, fieldMap: [FieldMap!]!)` directive have their fields re-exposed on other objects. For example, consider this schema:

```graphql
input SetUserNameParams {
  id: ID!
  some_other_param: String!
}

type SetUserNameResponse {
  updated_user: User!
}

type Mutation
  @exposeField(
    field: "set_user_name" # expose this field
    path: "updated_user" # on the type at this path (relative to the response object)
    fieldMap: [{ from: "id", to: "id" }] # mapping these fields
    # as: "custom_field_name"
  ) {
  set_user_name(input: SetUserNameParams!): SetUserNameResponse!
}
```

In the above example, the `set_user_name` field will be made available on every `User` object, under the key `set_user_name` (this will be customizable.) So, one could write a resolver:

```jsx
export const UpdateUserNameButton = iso(`
  field User.UpdateUserNameButton {
    set_user_name
  }
`)((data) => {
  return (
    <div
      onClick={() => data.set_user_name({ input: { new_name: 'Maybe' } })[1]()}
    >
      Call me, maybe
    </div>
  );
});
```

Clicking that button executes a mutation. The `id` field is automatically passed in (i.e. it comes from whatever `User` object where this field was selected.)

The fields that are refetched as part of the mutation response are whatever fields are selected on that user in the _merged_ query! So, if on that same `User`, we also (potentially through another resolver) selected the `name` field, the mutation response would include `name`! If, later, we selected `email`, it would also be fetched.

## Getting involved and learning more

There's a lot more. These docs are threadbare.

- See the sample apps in [`./demos`](./demos/).
- Watch the [talk at GraphQL Conf](https://www.youtube.com/watch?v=gO65JJRqjuc).
- Join the [Discord](https://discord.gg/rjDDwvZR).
- [Follow the official Twitter account](https://twitter.com/isographlabs)

## Other, older resources

- See [the developer experience of using Isograph](https://www.youtube.com/watch?v=f1nfXc3VeTk).
- Read [the substack article](https://isograph.substack.com/p/introducing-isograph).

## Licensing

Isograph is an open source software project and licensed under the terms of the MIT license.
