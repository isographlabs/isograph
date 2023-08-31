import type {IsographNonFetchableResolver, ReaderAst} from '@isograph/react';
import { user_detail as resolver } from '../../user_detail.tsx';
import User__repository_list, { ReadOutType as User__repository_list__outputType } from '../User/repository_list.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (additionalRuntimeProps: Object | void) => (React.ReactElement<any, any> | null);

// TODO support changing this
export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Linked",
    fieldName: "user",
    alias: null,
    arguments: [
      {
        argumentName: "login",
        variableName: "userLogin",
      },
    ],
    selections: [
      {
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
      },
      {
        kind: "Resolver",
        alias: "repository_list",
        arguments: null,
        resolver: User__repository_list,
        variant: "Component",
        usedRefetchQueries: [0],
        // This should only exist on refetch queries
        refetchQuery: 0,
      },
    ],
  },
];

export type ResolverParameterType = { data:
{
  user: ({
    name: (string | null),
    repository_list: User__repository_list__outputType,
  } | null),
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
