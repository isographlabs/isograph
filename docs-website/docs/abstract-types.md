# Refining from abstract to concrete types

Fields with an abstract type (e.g. `node`) have special `asConcreteType` fields. These will be null if the field does not have that concrete type.

For example:

```jsx
export const UserLink = iso(`
  field Actor.ActorGreeting @component {
    login
    asUser {
      twitterUsername
    }
  }
`)(function UserLinkComponent({ data }) {
  return (
    <>
      Hello <b>{data.login}</b>
      {data.asUser != null
        ? ` (who goes by ${data.asUser.twitterUsername ?? ''} on Twitter!)`
        : null}
    </>
  );
});
```

The above code will result in the following query text generated:

```graphql
login
... on User {
  twitterUsername
}
```

## Data-driven dependencies

Check out the [data driven dependencies](/docs/data-driven-dependencies/) documentation to see how to combine [`@loadable` fields](/docs/loadable-fields/), [pagination](/docs/pagination/) and `asConcreteType` fields to fetch the minimal amount of data and JavaScript needed!
