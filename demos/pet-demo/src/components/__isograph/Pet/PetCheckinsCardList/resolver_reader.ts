import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Pet__PetCheckinsCardList__param } from './param_type';
import { Pet__PetCheckinsCardList__output_type } from './output_type';
import { PetCheckinsCardList as resolver } from '../../../PetCheckinsCard';
import Checkin__CheckinDisplay__resolver_reader from '../../Checkin/CheckinDisplay/resolver_reader';

const readerAst: ReaderAst<Pet__PetCheckinsCardList__param> = [
  {
    kind: "Linked",
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
        fieldName: "id",
        alias: null,
        arguments: null,
      },
    ],
  },
];

const artifact: EagerReaderArtifact<
  Pet__PetCheckinsCardList__param,
  Pet__PetCheckinsCardList__output_type
> = {
  kind: "EagerReaderArtifact",
  resolver,
  readerAst,
};

export default artifact;
