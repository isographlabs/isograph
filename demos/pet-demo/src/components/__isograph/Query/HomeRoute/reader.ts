import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { HomeRoute as resolver } from '../../../HomeRoute.tsx';
import Pet__PetSummaryCard, { Pet__PetSummaryCard__outputType} from '../../Pet/PetSummaryCard/reader';

// the type, when read out (either via useLazyReference or via graph)
export type Query__HomeRoute__outputType = (React.FC<any>);

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

export type Query__HomeRoute__param = { data:
{
  pets: ({
    id: string,
    PetSummaryCard: Pet__PetSummaryCard__outputType,
  })[],
},
[index: string]: any };

const artifact: ReaderArtifact<
  Query__HomeRoute__param,
  Query__HomeRoute__param,
  Query__HomeRoute__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.HomeRoute" },
};

export default artifact;
