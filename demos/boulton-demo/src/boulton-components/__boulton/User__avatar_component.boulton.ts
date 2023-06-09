import type {BoultonNonFetchableResolver, ReaderAst} from '@boulton/react';
import { avatar_component as resolver } from '../avatar.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = React.FC<{ } & Object>;

// TODO support changing this
export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Scalar",
    response_name: "name",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    response_name: "email",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    response_name: "avatar_url",
    alias: null,
    arguments: null,
  },
];

export type ResolverParameterType = { data:
{
  name: string,
  email: string,
  avatar_url: string,
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: BoultonNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'NonFetchableResolver',
  resolver: resolver as any,
  readerAst,
// is this needed?
              convert: (x) => {throw new Error('convert non fetchable')},
};

export default artifact;
