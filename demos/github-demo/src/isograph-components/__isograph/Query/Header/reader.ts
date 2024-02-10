import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Header as resolver } from '../../../header.tsx';
import User__Avatar, { ReadOutType as User__Avatar__outputType } from '../../User/Avatar/reader';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (React.FC<any>);

export type ReadFromStoreType = Query__Header__param;

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
        alias: "Avatar",
        arguments: null,
        readerArtifact: User__Avatar,
        usedRefetchQueries: [],
      },
    ],
  },
];

export type Query__Header__param = { data:
{
  viewer: {
    name: (string | null),
    Avatar: User__Avatar__outputType,
  },
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: ReaderArtifact<ReadFromStoreType, Query__Header__param, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.Header" },
};

export default artifact;
