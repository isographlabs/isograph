import type {IsographNonFetchableResolver, ReaderAst} from '@isograph/react';
import { pet_stats_card as resolver } from '../../../components/pet_stats_card.tsx';
import Pet____refetch, { ReadOutType as Pet____refetch__outputType } from '../__refetch/reader.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (additionalRuntimeProps: Object | void) => (React.ReactElement<any, any> | null);

export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "nickname",
    alias: null,
    arguments: null,
  },
  {
    kind: "RefetchField",
    alias: "__refetch",
    readerArtifact: Pet____refetch,
    refetchQuery: 0,
  },
  {
    kind: "Scalar",
    fieldName: "age",
    alias: null,
    arguments: null,
  },
  {
    kind: "Linked",
    fieldName: "stats",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "weight",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "intelligence",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "cuteness",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "hunger",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "sociability",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "energy",
        alias: null,
        arguments: null,
      },
    ],
  },
];

export type ResolverParameterType = { data:
{
  id: string,
  nickname: (string | null),
  __refetch: Pet____refetch__outputType,
  age: number,
  stats: {
    weight: number,
    intelligence: number,
    cuteness: number,
    hunger: number,
    sociability: number,
    energy: number,
  },
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: IsographNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'NonFetchableResolver',
  resolver: resolver as any,
  readerAst,
};

export default artifact;
