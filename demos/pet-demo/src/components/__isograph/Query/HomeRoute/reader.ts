import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { HomeRoute as resolver } from '../../../HomeRoute.tsx';
import Pet__PetSummaryCard, { ReadOutType as Pet__PetSummaryCard__outputType } from '../../Pet/PetSummaryCard/reader';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (React.FC<any>);

export type ReadFromStoreType = Query__HomeRoute__param;

const readerAst: ReaderAst<ReadFromStoreType> = [
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

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: ReaderArtifact<ReadFromStoreType, Query__HomeRoute__param, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.HomeRoute" },
};

export default artifact;
