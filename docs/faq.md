# FAQ

## Why do `@component` resolvers need to be interpolated into the parent, and not rendered?

Consider a parent component that fetches a child component. The child component should be interpolated into the returned JSX:

```js
export const parent_component = iso`
  Query.parent_component @component {
    child_component,
  }
`(function ParentComponent({ data }) {
  // THIS IS CORRECT:
  return (
    <>
      Parent component
      {child_component({})}
    </>
  );
  // AND THIS IS NOT:
  // return <>
  //   Parent component
  //   <child_component />
  // </>
});
```

This is because the `child_component` function is **not** referentially stable. In particular, every time `data` changes, the component would have to lose its state. However, the value returned from `child_component({})` **is** an element whose component is referentially stable, meaning that the component will not lose its state when data changes, and thus **de facto** is indistinguishable from a regular component.
