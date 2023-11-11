import type {IsographNonFetchableResolver, ReaderAst} from '@isograph/react';
const resolver = (x: any) => x;
import Pet__pet_summary_card, { ReadOutType as Pet__pet_summary_card__outputType } from '../../Pet/pet_summary_card/reader.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = ResolverReturnType;

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

export type ResolverParameterType = {
  pets: ({
    id: string,
    pet_summary_card: Pet__pet_summary_card__outputType,
  })[],
};

// The type, when returned from the resolver
export type ResolverReturnType = ResolverParameterType;

const artifact: IsographNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: "NonFetchableResolver",
  resolver: resolver as any,
  readerAst,
  variant: "Eager",
};

export default artifact;
