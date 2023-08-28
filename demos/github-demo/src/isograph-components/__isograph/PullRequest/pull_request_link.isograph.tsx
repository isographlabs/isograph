import type {IsographNonFetchableResolver, ReaderAst} from '@isograph/react';
import { pull_request_link as resolver } from '../../pull_request_link.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (additionalRuntimeProps: Object | void) => (React.ReactElement<any, any> | null);

// TODO support changing this
export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Scalar",
    fieldName: "number",
    alias: null,
    arguments: null,
  },
  {
    kind: "Linked",
    fieldName: "repository",
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
        kind: "Linked",
        fieldName: "owner",
        alias: null,
        arguments: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "login",
            alias: null,
            arguments: null,
          },
        ],
      },
    ],
  },
];

export type ResolverParameterType = { data:
{
  number: number,
  repository: {
    name: string,
    owner: {
      login: string,
    },
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
