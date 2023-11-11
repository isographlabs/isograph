import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { home_route as resolver } from '../../../components/home_route.tsx';
import Pet__pet_summary_card, { ReadOutType as Pet__pet_summary_card__outputType } from '../../Pet/pet_summary_card/reader.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (additionalRuntimeProps: Object | void) => (React.ReactElement<any, any> | null);

export type ReadFromStoreType = ResolverParameterType;

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
        alias: "pet_summary_card",
        arguments: null,
        readerArtifact: Pet__pet_summary_card,
        usedRefetchQueries: [],
      },
    ],
  },
];

export type ResolverParameterType = { data:
{
  pets: ({
    id: string,
    pet_summary_card: Pet__pet_summary_card__outputType,
  })[],
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: ReaderArtifact<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: "Component",
};

export default artifact;
