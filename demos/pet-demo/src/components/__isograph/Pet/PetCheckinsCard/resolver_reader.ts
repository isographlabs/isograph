import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Pet__PetCheckinsCard__param } from './param_type';
import { PetCheckinsCard as resolver } from '../../../Pet/PetCheckinsCard';
import Checkin__CheckinDisplay__resolver_reader from '../../Checkin/CheckinDisplay/resolver_reader';

const readerAst: ReaderAst<Pet__PetCheckinsCard__param> = [
  {
    kind: "Scalar",
    isFallible: false,
    fieldName: "id",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Linked",
    isFallible: false,
    fieldName: "checkins",
    alias: null,
    arguments: [
      [
        "skip",
        { kind: "Variable", name: "skip" },
      ],

      [
        "limit",
        { kind: "Variable", name: "limit" },
      ],
    ],
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Resolver",
        alias: "CheckinDisplay",
        arguments: null,
        readerArtifact: Checkin__CheckinDisplay__resolver_reader,
        usedRefetchQueries: [0, ],
      },
      {
        kind: "Scalar",
        isFallible: false,
        fieldName: "id",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
    ],
  },
];

const artifact = (): ComponentReaderArtifact<
  Pet__PetCheckinsCard__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "PetCheckinsCard",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
