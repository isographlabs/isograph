import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { header as resolver } from '../../../isograph-components/header.tsx';
import User__avatar, { ReadOutType as User__avatar__outputType } from '../../User/avatar/reader.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (React.FC<any>);

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
        readerArtifact: User__avatar,
        usedRefetchQueries: [],
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

const artifact: ReaderArtifact<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.header" },
};

export default artifact;
