import type {ReaderArtifact, ReaderAst, ExtractSecondParam} from '@isograph/react';
import { Header as resolver } from '../../../header.tsx';
import User__Avatar, { User__Avatar__outputType} from '../../User/Avatar/reader';

// the type, when read out (either via useLazyReference or via graph)
export type Query__Header__outputType = (React.FC<ExtractSecondParam<typeof resolver>>);

const readerAst: ReaderAst<Query__Header__param> = [
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

export type Query__Header__param = {
  viewer: {
    name: (string | null),
    Avatar: User__Avatar__outputType,
  },
};

const artifact: ReaderArtifact<
  Query__Header__param,
  Query__Header__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.Header" },
};

export default artifact;
