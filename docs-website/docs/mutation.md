# Mutations

In Isograph, mutations aren't special. The distinguishing feature of mutations is often that you want to make the network request at a specific time, for example, in response to a user clicking a button. This document describes how to make network requests in response to events, which you can use to trigger a mutation. Okay, onward!

:::note
Examples in this document are taken from [this file](https://github.com/isographlabs/isograph/blob/91d3020f7f28a9cd91c250a9457f6a6bc7fd1562/demos/pet-demo/src/components/PetTaglineCard.tsx).
:::

## Walkthrough

### Defining the mutation field

First, define a client field on the `Mutation` object that calls the mutation you care about:

```js
export const setTagline = iso(`
  field Mutation.SetTagline($input: SetPetTaglineParams!) {
    set_pet_tagline(input: $input) {
      pet {
        tagline
      }
    }
  }
`)((()) => {});
```

Make sure you select the fields that you want refetched and written into the store. In this case, we want to see the updated pet's tagline. We also return the data.

:::note
Note also that we're naming the field `Mutation.SetTagline`, _not_ `Mutation.set_pet_tagline`. There already is a field named `Mutation.set_pet_tagline`, defined in the schema. So, if you attempt to define a client field named `Mutation.set_pet_tagline`, the Isograph compiler will emit an error and refuse to compile!
:::

### Calling the mutation

Next, call `useImperativeReference`. This gives you back a fragment reference and a `loadFragmentReference` function:

```jsx
import { useImperativeReference } from '@isograph/react';
import { UNASSIGNED_STATE } from '@isograph/react-disposable-state';

export const PetTaglineCard = iso(`
  field Pet.PetTaglineCard @component {
    id
    tagline
  }
`)(function PetTaglineCardComponent({ data: pet }) {
  const {
    fragmentReference: mutationRef,
    loadFragmentReference: loadMutation,
  } = useImperativeReference(iso(`entrypoint Mutation.SetTagline`));
  // ...
}
```

Next, call `loadFragmentReference` (`loadMutation` in this example) when a user clicks!

```jsx
{
  mutationRef === UNASSIGNED_STATE ? (
    <Button
      onClick={() => {
        loadMutation({
          input: {
            id: pet.id,
            tagline: 'SUPER DOG',
          },
        });
      }}
      variant="contained"
    >
      Set tagline to SUPER DOG
    </Button>
  ) : null;
}
```

Since we only want to set the tagline once, we check that `mutation === UNASSIGNED_STATE` before showing the button.

### Reading the results

What about reading the results of the mutation? There are two good ways to do this, and two additional ways that will work in the future.

First, we can wait until the network response completes and see the component re-render with the updated tagline.

For the second method, we can modify the mutation field as follows:

```js
export const setTagline = iso(`
  field Mutation.SetTagline($input: SetPetTaglineParams!) @component {
    set_pet_tagline(input: $input) {
      pet {
        tagline
      }
    }
  }
`)((({data})) => {
  return (
    <>
      Nice! You updated the pet's tagline to{' '}
      {data.set_pet_tagline?.pet?.tagline}!
    </>
  );
});
```

Here, we add the `@component` annotation and return some JSX.

Now, we can use this! Modify the `PetTaglineCardComponent` component as follows:

```js
{
  mutationRef === UNASSIGNED_STATE ? (
    <Button
      onClick={() => {
        loadMutation({
          input: {
            id: pet.id,
            tagline: 'SUPER DOG',
          },
        });
      }}
      variant="contained"
    >
      Set tagline to SUPER DOG
    </Button>
  ) : (
    <Suspense fallback="Mutation in flight">
      <FragmentReader fragmentReference={mutationRef} />
    </Suspense>
  );
}
```

#### Future methods

The following methods will be available in the future:

- When reading a fragment reference is not a hook (see [this issue](https://github.com/isographlabs/isograph/issues/273)), you should be able to read the fragment conditionally in the parent component.

### Being notified when a mutation completes

The `loadFragmentReference` (aka `loadMutation` in these docs) function accepts a `FetchOptions` object, which can include an `onCompleted` callback, e.g.:

```jsx
loadMutation(
  {
    input: {
      id: pet.id,
      tagline: 'SUPER DOG',
    },
  },
  {
    onCompleted: () => {
      console.log('Done setting');
    },
  },
);
```

This `onCompleted` callback takes no parameters. **If you need to receive the results of the mutation, please comment on [this issue](https://github.com/isographlabs/isograph/issues/277).**

### Being notified when a mutation errors out

`FetchOptions` also accepts an `onError` callback:

```jsx
loadMutation(
  {
    input: {
      id: pet.id,
      tagline: 'SUPER DOG',
    },
  },
  {
    onError: () => {
      console.log('Oops');
    },
  },
);
```

### Appending to a list in response to a mutation

There are no good APIs to do this, currently. If you need this behavior, please [comment on this issue](https://github.com/isographlabs/isograph/issues/278) or reach out in the Discord. We're happy to help you out!
