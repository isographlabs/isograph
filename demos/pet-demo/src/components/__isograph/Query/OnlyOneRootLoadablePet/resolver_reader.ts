import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__OnlyOneRootLoadablePet__param } from './param_type';
import { OnlyOneRootLoadable as resolver } from '../../../Pet/PetWithOneRootLoadable';
import Query__PetFavoritePhrase2__entrypoint from '../../Query/PetFavoritePhrase2/entrypoint';

const readerAst: ReaderAst<Query__OnlyOneRootLoadablePet__param> = [
  {
    kind: "LoadablySelectedField",
    alias: "PetFavoritePhrase2",
    name: "PetFavoritePhrase2",
    queryArguments: [
      [
        "id",
        { kind: "Variable", name: "id" },
      ],
    ],
    refetchReaderAst: [
    ],
    entrypoint: Query__PetFavoritePhrase2__entrypoint,
  },
];

const artifact = (): ComponentReaderArtifact<
  Query__OnlyOneRootLoadablePet__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "OnlyOneRootLoadablePet",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
