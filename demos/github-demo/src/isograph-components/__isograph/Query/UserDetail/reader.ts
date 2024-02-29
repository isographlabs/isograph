import type {ReaderArtifact, ReaderAst, ExtractSecondParam} from '@isograph/react';
import { UserDetail as resolver } from '../../../UserDetail.tsx';
import User__RepositoryList, { User__RepositoryList__outputType} from '../../User/RepositoryList/reader';

// the type, when read out (either via useLazyReference or via graph)
export type Query__UserDetail__outputType = (React.FC<ExtractSecondParam<typeof resolver>>);

const readerAst: ReaderAst<Query__UserDetail__param> = [
  {
    kind: "Linked",
    fieldName: "user",
    alias: null,
    arguments: [
      [
        "login",
        { kind: "Variable", name: "userLogin" },
      ],
    ],
    selections: [
      {
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
      },
      {
        kind: "Resolver",
        alias: "RepositoryList",
        arguments: null,
        readerArtifact: User__RepositoryList,
        usedRefetchQueries: [],
      },
    ],
  },
];

export type Query__UserDetail__param = {
  user: ({
    name: (string | null),
    RepositoryList: User__RepositoryList__outputType,
  } | null),
};

const artifact: ReaderArtifact<
  Query__UserDetail__param,
  Query__UserDetail__param,
  Query__UserDetail__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.UserDetail" },
};

export default artifact;
