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
    response_name: "user",
    alias: null,
    arguments: {
      "login": "userLogin",
    },
    selections: [
      {
        kind: "Scalar",
        response_name: "id",
        alias: "foo",
        arguments: null,
      },
      {
        kind: "Scalar",
        response_name: "name",
        alias: null,
        arguments: null,
      },
      {
        kind: "Resolver",
        alias: "repository_list",
        arguments: null,
        resolver: User__repository_list,
        variant: "Component",
      },
    ],
  },
];

export type ResolverParameterType = { data:
{
  user: ({
    foo: string,
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
