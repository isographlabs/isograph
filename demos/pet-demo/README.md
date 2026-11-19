# pet-demo

pet-demo is an i2 project for trying the `isograph` daemon and the VS Code extension. Every iso literal in `src/` is meant to be valid against the schema in `schema/`, so a diagnostic in this project is either a bug or a feature i2 does not support yet.

## Opening it

From the i2 repo root:

```sh
pnpm open-pet-demo
```

The command builds `target/debug/isograph`, builds the extension, restarts a running pet-demo daemon, and opens this directory in VS Code with the extension loaded from source. `vscode-extension/README.md` describes the script and the restart workflow.

## Logging the entrypoint artifacts

From the i2 repo root, after `pnpm install`:

```sh
pnpm --filter pet-demo log-entrypoints
```

`scripts/log-entrypoints.cjs` registers `@babel/register` with `@babel/preset-typescript`, the local `@isograph/babel-plugin`, and `@babel/plugin-transform-modules-commonjs`, so Node compiles each file under `src/` as it is required. The script then requires `HomeRoute`, `PetDetailRoute`, `SpeciesRoute`, and `SetTagline`. Each exports its entrypoint literal as `<Module>Entrypoint`, and the Babel plugin replaces each ``iso(`entrypoint ...`)`` call with an import of the generated `entrypoint.ts` in `src/__isograph/`. The script logs each entrypoint artifact with its `readerArtifact` called:

```text
{
  kind: 'Entrypoint',
  name: 'Query.HomeRoute',
  queryText: 'query HomeRoute {\n' + ...,
  normalizationAst: { discriminatorResponseKey: '__typename', root: { entityName: 'Query', id: '__ROOT' }, selections: [ [Object] ] },
  readerArtifact: { readerAst: [ [Object] ], resolver: [Function: HomeRoute] }
}
```

and the same shape for `Query.PetDetailRoute`, `Query.SpeciesRoute`, and `Mutation.SetTagline`. An entrypoint's `readerArtifact` is a function that returns the field's reader artifact, so that loading the entrypoint artifact does not read the reader artifact's module, which imports the field's source file.

The script reads the committed artifacts. It does not run the daemon, so after changing a literal, run the daemon in this directory first.

## Config

`isograph.config.json` sets `source_files` to `src/**/*.ts`, `schema` to `schema/**/*.graphql`, and `output_directory` to `src`, so the daemon writes artifacts into `src/__isograph/`. That folder is generated and committed: running the daemon in this directory rewrites it. It holds `iso.ts`, `tsconfig.json`, a param type per field, a parameters type per field with variables, output types and reader artifacts per upstream's reachability rules, an entrypoint, normalization AST, `query_text.ts`, and `query_text.graphql` per entrypoint, and a link type per entity a field is declared on or a pointer points to.

## Schema

- `schema/scalars.graphql` declares `Int`, `Float`, `String`, `Boolean`, `ID`, and `Url`. i2 has no built-in scalars, so the schema declares them, as in `docs-website/docs/design-docs/mental-model.md`. The GraphQL spec says a schema document omits the built-in scalars; graphql-js still accepts these declarations.
- `schema/schema.graphql` has a `schema` definition, `Query`, `Mutation`, the interfaces `Node`, `Namable`, and `ICheckin`, the types `Pet`, `BestFriendRelationship`, `PetStats`, `Checkin`, `Viewer`, `BlogItem`, `AdItem`, and `Image`, the union `NewsfeedItem`, the enum `PetSpecies`, and the input `SetPetTaglineParams`. Most types and fields have descriptions.
- `schema/extensions.graphql` defines the directive `@derivedFrom`, extends `Pet` with `species` and `favorite_phrase`, extends `Query` with `petsBySpecies`, and extends `PetSpecies` with `HAMSTER`.

## Sources

`iso` is generated into `src/__isograph/iso.ts`; resolvers are typed by their field's generated param type. A field literal's overload returns upstream's `IdentityWithParam<P__f__param>`, so a resolver's `{ data }` parameter needs no annotation: `data` has the type of the field's selection set, a client field selected by another field has the type its resolver returns, and a pointer's resolver must return a link (`{ entityName, id }`, `Link<"E">` from `@isograph/runtime`) of its target. An entrypoint literal has the type of its generated entrypoint artifact. Every file in `src/` calls `iso(...)`. `tsc --noEmit --strict` over `src` passes. `picture` and `picture_together` select the custom scalar `Url`, which is `unknown` because the `isograph` binary's profile maps no custom scalars. Each `field` literal is exported under its field name and immediately called with a resolver function. Each entrypoint literal's artifact is logged.

- `src/Pet.ts`: `field Pet.fullName` with a string description, and `field Pet.bestFriend to Pet` with a nested selection set, whose resolver returns a link to a `Pet` or `null`.
- `src/Owner.ts`: `entity Owner`, which exists only in the sources; `field Owner.displayName` with no selection set; `field Owner.OwnerBadge`, which selects `displayName`; and `field Pet.owner to Owner!`, whose resolver returns a link to an `Owner`.
- `src/PetSummaryCard.ts`: `field Pet.PetSummaryCard` with a block-string description, selecting the source field `fullName` and the extension field `species`.
- `src/PetStatsCard.ts`: `field Pet.PetStatsCard` with a nested selection on `stats`.
- `src/PetCheckinsCard.ts`: `field Pet.PetCheckinsCard($limit: Int = 3)`, passing `$limit` and the literal `0` to `checkins`.
- `src/PetBestFriendCard.ts`: `field Pet.PetBestFriendCard` with two levels of nesting and the aliased selection `friendName: firstName`.
- `src/HomeRoute.ts`: `field Query.HomeRoute`, selecting `PetSummaryCard`, and `entrypoint Query.HomeRoute`.
- `src/PetDetailRoute.ts`: `field Query.PetDetailRoute($id: ID!)`, selecting `pet(id: $id)` and under it the source fields `fullName`, `PetStatsCard`, `PetCheckinsCard(limit: 5)`, `PetBestFriendCard`, `bestFriend { fullName }`, and `owner { OwnerBadge }`; and `entrypoint Query.PetDetailRoute`.
- `src/SpeciesRoute.ts`: `field Query.SpeciesRoute($species: PetSpecies!)`, selecting the extension field `petsBySpecies(species: $species)`, `viewer`, and `node(id: "1")` on the interface `Node`; and `entrypoint Query.SpeciesRoute`.
- `src/SetTagline.ts`: `field Mutation.SetTagline($id: ID!, $tagline: String!)`, passing the object literal `{ id: $id, tagline: $tagline }` to `set_pet_tagline`; and `entrypoint Mutation.SetTagline`.

## Not yet validated

i2 parses every literal here. It does not yet check them against the schema. These literals use the following features, which need checks beyond looking up each selection's selectable on its entity:

- Directives (`@component`, `@loadable(lazyLoadArtifact: true)`). The schema does not declare them; they are isograph directives, not GraphQL directives.
- Argument values against argument types: `$limit: Int` passed to `checkins(limit:)`, the object literal passed to the input `SetPetTaglineParams`, `"1"` passed to `ID!`.
- Variable default values (`$limit: Int = 3`).
- The selection set of a `to` field computing a value of the target entity (`Pet.bestFriend`, `Pet.owner`).
- Selections on the union `NewsfeedItem`. No literal selects one, because the iso grammar has no inline fragment or type refinement.
