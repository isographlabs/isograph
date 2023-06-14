import type {IsographNonFetchableResolver, ReaderAst} from '@isograph/react';
import { user_detail as resolver } from '../user_list_page.tsx';
import User__avatar_component, { ReadOutType as User__avatar_component__outputType } from './User__avatar_component.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (additionalRuntimeProps: Object | void) => (React.ReactElement<any, any> | null);

// TODO support changing this
export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
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
    alias: "avatar_component",
    arguments: null,
    resolver: User__avatar_component,
    variant: "Component",
  },
];

export type ResolverParameterType = { data:
{
  id: string,
  name: string,
  avatar_component: User__avatar_component__outputType,
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
