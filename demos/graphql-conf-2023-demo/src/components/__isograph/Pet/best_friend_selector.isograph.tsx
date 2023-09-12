import type {IsographNonFetchableResolver, ReaderAst} from '@isograph/react';
import { best_friend_selector as resolver } from '../../best_friend_selector.tsx';
import Pet____set_pet_best_friend, { ReadOutType as Pet____set_pet_best_friend__outputType } from './__set_pet_best_friend.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (additionalRuntimeProps: Object | void) => (React.ReactElement<any, any> | null);

export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "MutationField",
    alias: "__set_pet_best_friend",
    resolver: Pet____set_pet_best_friend,
    refetchQuery: 0,
  },
  {
    kind: "Linked",
    fieldName: "potential_new_best_friends",
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
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
      },
    ],
  },
];

export type ResolverParameterType = { data:
{
  __set_pet_best_friend: Pet____set_pet_best_friend__outputType,
  potential_new_best_friends: ({
    id: string,
    name: string,
  })[],
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
