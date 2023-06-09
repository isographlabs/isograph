import type {BoultonNonFetchableResolver, ReaderAst} from '@boulton/react';
import { user_list_component as resolver } from '../user_list.tsx';
import User__avatar_component, { ReadOutType as User__avatar_component__outputType } from './User__avatar_component.boulton';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = React.FC<{ } & Object>;

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

const artifact: BoultonNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'NonFetchableResolver',
  resolver: resolver as any,
  readerAst,
  convert: (x) => x,
};

export default artifact;
