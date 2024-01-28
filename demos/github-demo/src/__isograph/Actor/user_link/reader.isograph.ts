import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { user_link as resolver } from '../../../isograph-components/user_link.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (React.FC<any>);

export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Scalar",
    fieldName: "login",
    alias: null,
    arguments: null,
  },
];

export type ResolverParameterType = { data:
{
  login: string,
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: ReaderArtifact<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Actor.user_link" },
};

export default artifact;
