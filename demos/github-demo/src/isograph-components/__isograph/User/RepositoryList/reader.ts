import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { RepositoryList as resolver } from '../../../UserRepositoryList.tsx';
import Repository__RepositoryLink, { Repository__RepositoryLink__outputType} from '../../Repository/RepositoryLink/reader';

// the type, when read out (either via useLazyReference or via graph)
export type User__RepositoryList__outputType = (React.FC<any>);

const readerAst: ReaderAst<User__RepositoryList__param> = [
  {
    kind: "Linked",
    fieldName: "repositories",
    alias: null,
    arguments: [
      [
        "last",
        { kind: "Literal", value: "10" },
      ],
    ],
    selections: [
      {
        kind: "Linked",
        fieldName: "edges",
        alias: null,
        arguments: null,
        selections: [
          {
            kind: "Linked",
            fieldName: "node",
            alias: null,
            arguments: null,
            selections: [
              {
                kind: "Scalar",
                fieldName: "id",
                alias: null,
                arguments: null,
              },
              {
                kind: "Resolver",
                alias: "RepositoryLink",
                arguments: null,
                readerArtifact: Repository__RepositoryLink,
                usedRefetchQueries: [],
              },
              {
                kind: "Scalar",
                fieldName: "name",
                alias: null,
                arguments: null,
              },
              {
                kind: "Scalar",
                fieldName: "nameWithOwner",
                alias: null,
                arguments: null,
              },
              {
                kind: "Scalar",
                fieldName: "description",
                alias: null,
                arguments: null,
              },
              {
                kind: "Scalar",
                fieldName: "forkCount",
                alias: null,
                arguments: null,
              },
              {
                kind: "Linked",
                fieldName: "pullRequests",
                alias: null,
                arguments: [
                  [
                    "first",
                    { kind: "Variable", name: "first" },
                  ],
                ],
                selections: [
                  {
                    kind: "Scalar",
                    fieldName: "totalCount",
                    alias: null,
                    arguments: null,
                  },
                ],
              },
              {
                kind: "Scalar",
                fieldName: "stargazerCount",
                alias: null,
                arguments: null,
              },
              {
                kind: "Linked",
                fieldName: "watchers",
                alias: null,
                arguments: [
                  [
                    "first",
                    { kind: "Variable", name: "first" },
                  ],
                ],
                selections: [
                  {
                    kind: "Scalar",
                    fieldName: "totalCount",
                    alias: null,
                    arguments: null,
                  },
                ],
              },
            ],
          },
        ],
      },
    ],
  },
];

export type User__RepositoryList__param = { data:
{
  repositories: {
    edges: (({
      node: ({
        id: string,
        RepositoryLink: Repository__RepositoryLink__outputType,
        name: string,
        nameWithOwner: string,
        description: (string | null),
        forkCount: number,
        pullRequests: {
          totalCount: number,
        },
        stargazerCount: number,
        watchers: {
          totalCount: number,
        },
      } | null),
    } | null))[],
  },
},
[index: string]: any };

const artifact: ReaderArtifact<
  User__RepositoryList__param,
  User__RepositoryList__param,
  User__RepositoryList__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "User.RepositoryList" },
};

export default artifact;
