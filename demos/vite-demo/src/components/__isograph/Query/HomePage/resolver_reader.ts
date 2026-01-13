import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__HomePage__param } from './param_type';
import { HomePage as resolver } from '../../../HomePage';
import Pokemon__Pokemon__resolver_reader from '../../Pokemon/Pokemon/resolver_reader';

const readerAst: ReaderAst<Query__HomePage__param> = [
  {
    kind: "Linked",
    fieldName: "getAllPokemon",
    alias: null,
    arguments: [
      [
        "take",
        { kind: "Literal", value: 232 },
      ],

      [
        "offset",
        { kind: "Literal", value: 93 },
      ],
    ],
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "key",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Scalar",
        fieldName: "forme",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Resolver",
        alias: "Pokemon",
        arguments: null,
        readerArtifact: Pokemon__Pokemon__resolver_reader,
        usedRefetchQueries: [],
      },
    ],
  },
];

const artifact = (): ComponentReaderArtifact<
  Query__HomePage__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "HomePage",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
