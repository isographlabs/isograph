import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__PetFavoritePhrase__param } from './param_type';
import { PetFavoritePhrase as resolver } from '../../../Pet/FavoritePhrase';
import Pet__fullName__resolver_reader from '../../Pet/fullName/resolver_reader';

const readerAst: ReaderAst<Query__PetFavoritePhrase__param> = [
  {
    kind: "Linked",
    isFallible: true,
    fieldName: "pet",
    alias: null,
    arguments: [
      [
        "id",
        { kind: "Variable", name: "id" },
      ],
    ],
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Resolver",
        alias: "fullName",
        arguments: null,
        readerArtifact: Pet__fullName__resolver_reader,
        usedRefetchQueries: [],
      },
      {
        kind: "Scalar",
        isFallible: true,
        fieldName: "favorite_phrase",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
    ],
  },
];

const artifact = (): ComponentReaderArtifact<
  Query__PetFavoritePhrase__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "PetFavoritePhrase",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
