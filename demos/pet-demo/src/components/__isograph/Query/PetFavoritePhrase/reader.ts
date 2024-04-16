import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Query__PetFavoritePhrase__param } from './param_type.ts';
import { Query__PetFavoritePhrase__outputType } from './output_type.ts';
import { PetFavoritePhrase as resolver } from '../../../FavoritePhrase.tsx';

const readerAst: ReaderAst<Query__PetFavoritePhrase__param> = [
  {
    kind: "Linked",
    fieldName: "pet",
    alias: null,
    arguments: [
      [
        "id",
        { kind: "Variable", name: "id" },
      ],
    ],
    selections: [
      {
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "favorite_phrase",
        alias: null,
        arguments: null,
      },
    ],
  },
];

const artifact: ReaderArtifact<
  Query__PetFavoritePhrase__param,
  Query__PetFavoritePhrase__outputType
> = {
  kind: "ReaderArtifact",
  fieldName: "PetFavoritePhrase",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.PetFavoritePhrase" },
};

export default artifact;
