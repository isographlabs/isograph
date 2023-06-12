import type {BoultonNonFetchableResolver, ReaderAst} from '@boulton/react';
import { header as resolver } from '../header.tsx';
import User__avatar, { ReadOutType as User__avatar__outputType } from './User__avatar.boulton';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (additionalRuntimeProps: Object | void) => (React.ReactElement<any, any> | null);

// TODO support changing this
export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Linked",
    response_name: "viewer",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Scalar",
        response_name: "id",
        alias: null,
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
        alias: "avatar",
        arguments: null,
        resolver: User__avatar,
        variant: "Component",
      },
    ],
  },
];

export type ResolverParameterType = { data:
{
  viewer: {
    id: string,
    name: (string | null),
    avatar: User__avatar__outputType,
  },
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: BoultonNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'NonFetchableResolver',
  resolver: resolver as any,
  readerAst,
};

export default artifact;
