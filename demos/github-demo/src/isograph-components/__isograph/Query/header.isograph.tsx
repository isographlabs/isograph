import type {IsographNonFetchableResolver, ReaderAst} from '@isograph/react';
import { header as resolver } from '../../header.tsx';
import User__avatar, { ReadOutType as User__avatar__outputType } from '../User/avatar.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (additionalRuntimeProps: Object | void) => (React.ReactElement<any, any> | null);

// TODO support changing this
export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Linked",
    fieldName: "viewer",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
      },
      {
        kind: "Resolver",
        alias: "avatar",
        arguments: null,
        resolver: User__avatar,
        variant: "Component",
        usedRefetchQueries: [0, ],
        // This should only exist on refetch queries
        refetchQuery: 0,
      },
    ],
  },
];

export type ResolverParameterType = { data:
{
  viewer: {
    name: (string | null),
    avatar: User__avatar__outputType,
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
