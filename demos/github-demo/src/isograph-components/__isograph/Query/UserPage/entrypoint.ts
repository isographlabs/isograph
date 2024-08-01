import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__UserPage__param} from './param_type';
import {Query__UserPage__output_type} from './output_type';
import readerResolver from './resolver_reader';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const queryText = 'query UserPage ($first: Int!, $userLogin: String!) {\
  user____login___l_null: user(login: null) {\
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
          pullRequests____first___l_null: pullRequests(first: null) {\
            totalCount,\
          },\
          stargazerCount,\
          watchers____first___l_null: watchers(first: null) {\
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
        { kind: "Literal", value: null },
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
                        { kind: "Literal", value: null },
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
                        { kind: "Literal", value: null },
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
  nestedRefetchQueries,
  readerArtifact: readerResolver,
};

export default artifact;
