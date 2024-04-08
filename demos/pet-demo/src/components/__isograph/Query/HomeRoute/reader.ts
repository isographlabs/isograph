import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Query__HomeRoute__param } from './param_type.ts';
import { Query__HomeRoute__outputType } from './output_type.ts';
import { HomeRoute as resolver } from '../../../HomeRoute.tsx';
import Pet__PetSummaryCard from '../../Pet/PetSummaryCard/reader';

const readerAst: ReaderAst<Query__HomeRoute__param> = [
  {
    kind: "Linked",
    fieldName: "pets",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        alias: null,
        arguments: null,
      },
      {
        kind: "Resolver",
        alias: "PetSummaryCard",
        arguments: null,
        readerArtifact: Pet__PetSummaryCard,
        usedRefetchQueries: [],
      },
    ],
  },
];

const artifact: ReaderArtifact<
  Query__HomeRoute__param,
  Query__HomeRoute__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.HomeRoute" },
};

export default artifact;
