import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__UserPage__param} from './param_type';
import {Query__UserPage__output_type} from './output_type';
import readerResolver from './resolver_reader';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const queryText = 'query UserPage ($first: Int!, $userLogin: String!) {\
  user____login___v_userLogin: user(login: $userLogin) {\
    id,\
    name,\
    repositories____last___l_10: repositories(last: 10) {\
      edges {\
        node {\
          id,\
          description,\
          forkCount,\
          name,\
          nameWithOwner,\
          owner {\
            id,\
            login,\
          },\
          pullRequests____first___v_first: pullRequests(first: $first) {\
            totalCount,\
          },\
          stargazerCount,\
          watchers____first___v_first: watchers(first: $first) {\
            totalCount,\
          },\
        },\
      },\
    },\
  },\
  viewer {\
    id,\
    avatarUrl,\
    name,\
  },\
}';

const normalizationAst: NormalizationAst = [
  {
    kind: "Linked",
    fieldName: "user",
    arguments: [
      [
        "login",
        { kind: "Variable", name: "userLogin" },
      ],
    ],
    selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "name",
        arguments: null,
      },
      {
        kind: "Linked",
        fieldName: "repositories",
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
            arguments: null,
            selections: [
              {
                kind: "Linked",
                fieldName: "node",
                arguments: null,
                selections: [
                  {
                    kind: "Scalar",
                    fieldName: "id",
                    arguments: null,
                  },
                  {
                    kind: "Scalar",
                    fieldName: "description",
                    arguments: null,
                  },
                  {
                    kind: "Scalar",
                    fieldName: "forkCount",
                    arguments: null,
                  },
                  {
                    kind: "Scalar",
                    fieldName: "name",
                    arguments: null,
                  },
                  {
                    kind: "Scalar",
                    fieldName: "nameWithOwner",
                    arguments: null,
                  },
                  {
                    kind: "Linked",
                    fieldName: "owner",
                    arguments: null,
                    selections: [
                      {
                        kind: "Scalar",
                        fieldName: "id",
                        arguments: null,
                      },
                      {
                        kind: "Scalar",
                        fieldName: "login",
                        arguments: null,
                      },
                    ],
                  },
                  {
                    kind: "Linked",
                    fieldName: "pullRequests",
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
                        arguments: null,
                      },
                    ],
                  },
                  {
                    kind: "Scalar",
                    fieldName: "stargazerCount",
                    arguments: null,
                  },
                  {
                    kind: "Linked",
                    fieldName: "watchers",
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
    ],
  },
  {
    kind: "Linked",
    fieldName: "viewer",
    arguments: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "avatarUrl",
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "name",
        arguments: null,
      },
    ],
  },
];
const artifact: IsographEntrypoint<
  Query__UserPage__param,
  Query__UserPage__output_type
> = {
  kind: "Entrypoint",
  queryText,
  normalizationAst,
  readerWithRefetchQueries: {
    kind: "ReaderWithRefetchQueries",
    nestedRefetchQueries,
    readerArtifact: readerResolver,
  },
};

export default artifact;
