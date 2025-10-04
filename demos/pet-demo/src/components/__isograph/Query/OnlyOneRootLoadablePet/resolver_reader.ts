import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__OnlyOneRootLoadablePet__param } from './param_type';
import { OnlyOneRootLoadable as resolver } from '../../../Pet/PetWithOneRootLoadable';
import Query__PetDetailRoute__resolver_reader from '../../Query/PetDetailRoute/resolver_reader';

const readerAst: ReaderAst<Query__OnlyOneRootLoadablePet__param> = [
  {
    kind: "Resolver",
    alias: "PetDetailRoute",
    arguments: [
      [
        "id",
        { kind: "Variable", name: "id" },
      ],
    ],
    readerArtifact: Query__PetDetailRoute__resolver_reader,
    usedRefetchQueries: [0, 1, 2, 3, 4, 5, ],
  },
];

const artifact: ComponentReaderArtifact<
  Query__OnlyOneRootLoadablePet__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "Query.OnlyOneRootLoadablePet",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
