import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Avatar as resolver } from '../../../isograph-components/Avatar.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (React.FC<any>);

export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Scalar",
    fieldName: "name",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "avatarUrl",
    alias: null,
    arguments: null,
  },
];

export type ResolverParameterType = { data:
{
  name: (string | null),
  avatarUrl: string,
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: ReaderArtifact<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "User.Avatar" },
};

export default artifact;
