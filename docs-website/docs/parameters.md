# Parameters

## Parameters and client fields

Unlike in GraphQL, there are no global variables (yet!) in Isograph. Instead, all parameters used in a client field must be defined in that client field, and no parameters can be unused. Example:

```jsx
export const PetDetailDeferredRouteComponent = iso(`
  field Query.PetDetailRoute($id: ID!) @component {
    pet(id: $id) {
      PetDetail
    }
  }
`)(function PetDetailRouteComponent({ data }) {
  // ...
});
```

## Accessing parameters at runtime

The parameters with which a client field was read can be accessed as part of that first parameter. For example:

```jsx
export const PetDetailDeferredRouteComponent = iso(`
  field Query.PetDetailRoute($id: ID!) @component {
    pet(id: $id) {
      PetDetail
    }
  }
`)(function PetDetailRouteComponent({ data, parameters }) {
  console.log('hello from pet ' + parameters.id + '!');
  // ...
});
```

## Loadable fields are variables

Parameters can be omitted in isograph literals from loadably selected client fields. Those missing variables become parameters that you must pass when making the network request (i.e. loading the field):

```jsx
export const BlogItem = iso(`
  field BlogItem.BlogItemDisplay @component {
    # Assume that BlogItemMoreDetail accepts an includeImages: Boolean! parameter
    # that we are skipping here, which is allowed because the field is selected
    # loadably:
    BlogItemMoreDetail @loadable(lazyLoadArtifact: true)
  }
`)(({ data: blogItem }) => {
  const { fragmentReference, loadFragmentReference } =
    useImperativeLoadableField(blogItem.BlogItemMoreDetail);

  const loadBlogItem = () =>
    loadFragmentReference({
      // now, includeImages must be passed here:
      includeImages: true,
    });
});
```

## Typechecking

There is no typechecking of variables, except to inasmuch as nullable variables are allowed to be missing. This feature is coming soon!
