import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__PetDetailRoute__param } from './param_type';
import { PetDetailRouteComponent as resolver } from '../../../PetDetailRoute';
import Query__PetDetailRouteInner__resolver_reader from '../../Query/PetDetailRouteInner/resolver_reader';

const readerAst: ReaderAst<Query__PetDetailRoute__param> = [
  {
    kind: "Resolver",
    alias: "PetDetailRouteInner",
    arguments: [
      [
        "actualId",
        { kind: "Variable", name: "id" },
      ],
    ],
    readerArtifact: Query__PetDetailRouteInner__resolver_reader,
    usedRefetchQueries: [0, 1, 2, 3, 4, ],
  },
];

const artifact: ComponentReaderArtifact<
  Query__PetDetailRoute__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Query.PetDetailRoute",
  resolver,
  readerAst,
};

export default artifact;
