# `@exposeField` directives

:::note
Schema extensions

You can include a `schema_extensions` field in your `isograph.config.json` file. It's value should be an array of schema extension files.

If the source of truth for your schema is not the repository where you use Isograph (e.g. it is imported from elsewhere, or it is generated, etc.), you may find it easier to work with a schema extension. That is what we will do in this guide.
:::

A mutation often has a primary object that is modified. For example, a `setUserProperties` mutation might modify a user, and return the updated user.

In Isograph, you can select that `setUserProperties` mutation directly off of a `User` object. This has several advantages:

- it's convenient! You'll often need user fields (e.g. the user's name) in the screen that modifies the user.
- since we know which user you're mutating, you don't have to pass the user's ID as a mutation parameter
- since the compiler knows what fields you fetched on that user, you can refetch those exact same fields in the mutation response!

## How do we create such fields?

Let's assume that we have a schema containing the following:

```graphql
type Mutation {
  set_pet_tagline(input: SetPetTaglineParams!): SetPetTaglineResponse!
}

input SetPetTaglineParams {
  id: ID!
  tagline: String!
}

type SetPetTaglineResponse {
  pet: Pet!
}

type Pet {
  id: ID!
  tagline: String
  # other fields
}
```

> The schema in the [`demos/pet-demo`](https://github.com/isographlabs/isograph/tree/main/demos/pet-demo) is like this.

We can add an `@exposeField` directive to the `Mutation` object to expose a magic mutation field as follows, by putting the following in our `schema-extension.graphql` file:

```graphql
extend type Mutation
  @exposeField(
    field: "set_pet_tagline"
    path: "pet"
    fieldMap: [{ from: "id", to: "input.id" }]
    as: "set_tagline"
  )
```

Let's go through each of these parameters in turn.

- `field` this is the field on the `Mutation` object that we want to expose.
- `path` this is the path in the mutation field's response object to the parent object, **on which we want to expose the field**. So, `SetPetTaglineResponse.pet` gets us a `Pet` object, so each `Pet` will have the magic mutation field added.
- `fieldMap`: this is an array of `from` and `to` values, that maps fields **from** the `Pet` **to** the mutation field params. So, we are mapping `Pet.id` to the `id` field of the `input` param of the `set_pet_tagline` field.
  - since this field is provided, this means that the user must provide something that looks like `{ input: { tagline } }`, and Isograph fills in the rest.
- `as`: the newly created field will have the name `set_tagline`. By default, the name of the new field will keep the name of the mutation field (i.e. `set_pet_tagline`).

## How do we use this field?

Now, this field (prefixed with two underscores) is available on the `Pet` object. (This prefix will be removed, and the exposed field name will be customizable.)

You might select it like

```tsx
export const PetTaglineInput = iso`
  Pet.PetTaglineInput @component {
    set_tagline,
    tagline,
  }
`(PetTaglineInputComponent);
```

When read out, the field is a function that when called will make a network request for the mutation. (Also, in the future, you will be able to do things like get the status of the mutation, suspend on it, etc. For now, it just triggers a mutation in the background.)

You might use it as follows:

```tsx
// Note: if you inline this function into the iso literal, you will not have to
// annotate the type of props. If it is defined separately, TypeScript will probably
// complain that it doesn't know what the type of data is.
function PetTaglineInputComponent(data) {
  const [tagline, setTagline] = useState<string>(data.tagline);

  const updateTagline = () => data.set_tagline({ input: { tagline } });

  return (
    <>
      <Input
        value={tagline}
        onChange={(e) => setTagline(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            updateTagline();
          }
        }}
      />
      <Button onClick={updateTagline}>Set tagline</Button>
    </>
  );
}
```

The fields selected on the mutation response (under the pet) will be **exactly the fields that are selected on that Pet in the merged query**. In other words, it will contain `tagline` (because that is selected in `Pet.PetTaglineInput`), `id` (which is automatically selected by Isograph) and any other fields that are selected by resolvers on the same pet in the same query.

You can view the generated mutation query by looking for a file whose name starts with `__refetch__`.

## We're just modifying the tagline! Why refetch the entire Pet?

A future version of Isograph will support refetching fewer fields.
