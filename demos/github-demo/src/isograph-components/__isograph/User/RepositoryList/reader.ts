import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { User__RepositoryList__param } from './param_type.ts';
import { User__RepositoryList__outputType } from './output_type.ts';
import { RepositoryList as resolver } from '../../../UserRepositoryList.tsx';
import Repository__RepositoryLink from '../../Repository/RepositoryLink/reader';

const readerAst: ReaderAst<User__RepositoryList__param> = [
  {
    kind: "Linked",
    fieldName: "repositories",
    alias: null,
    arguments: [
      [
        "last",
        { kind: "Literal", value: 10 },
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

const artifact: ReaderArtifact<
  User__RepositoryList__param,
  User__RepositoryList__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "User.RepositoryList" },
};

export default artifact;
