# pet-demo

pet-demo is an i2 project for trying the `isograph` daemon and the VS Code extension. Every iso literal in `src/` is meant to be valid against the schema in `schema/`, so a diagnostic in this project is either a bug or a feature i2 does not support yet.

## Opening it

From the i2 repo root:

```sh
pnpm open-pet-demo
```

The command builds `target/debug/isograph`, builds the extension, restarts a running pet-demo daemon, and opens this directory in VS Code with the extension loaded from source. `vscode-extension/README.md` describes the script and the restart workflow.

## Config

`isograph.config.json` sets `source_files` to `src/**/*.ts` and `schema` to `schema/**/*.graphql`.

## Schema

- `schema/scalars.graphql` declares `Int`, `Float`, `String`, `Boolean`, `ID`, and `Url`. i2 has no built-in scalars, so the schema declares them, as in `docs-website/docs/design-docs/mental-model.md`. The GraphQL spec says a schema document omits the built-in scalars; graphql-js still accepts these declarations.
- `schema/schema.graphql` has a `schema` definition, `Query`, `Mutation`, the interfaces `Node`, `Namable`, and `ICheckin`, the types `Pet`, `BestFriendRelationship`, `PetStats`, `Checkin`, `Viewer`, `BlogItem`, `AdItem`, and `Image`, the union `NewsfeedItem`, the enum `PetSpecies`, and the input `SetPetTaglineParams`. Most types and fields have descriptions.
- `schema/extensions.graphql` defines the directive `@derivedFrom`, extends `Pet` with `species` and `favorite_phrase`, extends `Query` with `petsBySpecies`, and extends `PetSpecies` with `HAMSTER`.

## Sources

`src/iso.ts` defines `iso`, which returns the resolver it is passed. Every other file calls `iso(...)`. Each `field` literal is exported under its field name and immediately called with a resolver function.

- `src/Pet.ts`: `field Pet.fullName` with a string description, and `field Pet.bestFriend to Pet` with a nested selection set.
- `src/Owner.ts`: `entity Owner`, which exists only in the sources; `field Owner.displayName` with no selection set; `field Owner.OwnerBadge`, which selects `displayName`; and `field Pet.owner to Owner!`.
- `src/PetSummaryCard.ts`: `field Pet.PetSummaryCard` with a block-string description, selecting the source field `fullName` and the extension field `species`.
- `src/PetStatsCard.ts`: `field Pet.PetStatsCard` with a nested selection on `stats`.
- `src/PetCheckinsCard.ts`: `field Pet.PetCheckinsCard($limit: Int = 3)`, passing `$limit` and the literal `0` to `checkins`.
- `src/PetBestFriendCard.ts`: `field Pet.PetBestFriendCard` with two levels of nesting and the aliased selection `friendName: firstName`.
- `src/HomeRoute.ts`: `field Query.HomeRoute`, selecting `PetSummaryCard`, and `entrypoint Query.HomeRoute`.
- `src/PetDetailRoute.ts`: `field Query.PetDetailRoute($id: ID!)`, selecting `pet(id: $id)` and under it the source fields `fullName`, `PetStatsCard`, `PetCheckinsCard(limit: 5) @loadable(lazyLoadArtifact: true)`, `PetBestFriendCard`, `bestFriend { fullName }`, and `owner { OwnerBadge }`; and `entrypoint Query.PetDetailRoute`.
- `src/SpeciesRoute.ts`: `field Query.SpeciesRoute($species: PetSpecies!)`, selecting the extension field `petsBySpecies(species: $species)`, `viewer`, and `node(id: "1")` on the interface `Node`; and `entrypoint Query.SpeciesRoute`.
- `src/SetTagline.ts`: `field Mutation.SetTagline($id: ID!, $tagline: String!)`, passing the object literal `{ id: $id, tagline: $tagline }` to `set_pet_tagline`; and `entrypoint Mutation.SetTagline`.

## Not yet validated

i2 parses every literal here. It does not yet check them against the schema. These literals use the following features, which need checks beyond looking up each selection's selectable on its entity:

- Directives (`@component`, `@loadable(lazyLoadArtifact: true)`). The schema does not declare them; they are isograph directives, not GraphQL directives.
- Argument values against argument types: `$limit: Int` passed to `checkins(limit:)`, the object literal passed to the input `SetPetTaglineParams`, `"1"` passed to `ID!`.
- Variable default values (`$limit: Int = 3`).
- The selection set of a `to` field computing a value of the target entity (`Pet.bestFriend`, `Pet.owner`).
- Selections on the union `NewsfeedItem`. No literal selects one, because the iso grammar has no inline fragment or type refinement.
