import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__RepositoryPage__param} from './param_type';
import {Query__RepositoryPage__output_type} from './output_type';
import readerResolver from './resolver_reader';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const queryText = 'query RepositoryPage ($repositoryName: String!, $repositoryOwner: String!, $first: Int!) {\
  repository____name___l_null____owner___l_null: repository(name: null, owner: null) {\
    id,\
    nameWithOwner,\
    parent {\
      id,\
      name,\
      nameWithOwner,\
      owner {\
        id,\
        login,\
      },\
    },\
    pullRequests____last___l_null: pullRequests(last: null) {\
      edges {\
        node {\
          id,\
          author {\
            login,\
          },\
          closed,\
          createdAt,\
          number,\
          repository {\
            id,\
            name,\
            owner {\
              id,\
              login,\
            },\
          },\
          title,\
          totalCommentsCount,\
        },\
      },\
    },\
    stargazerCount,\
    viewerHasStarred,\
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
    fieldName: "repository",
    arguments: [
      [
        "name",
        { kind: "Literal", value: null },
      ],

      [
        "owner",
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
        fieldName: "nameWithOwner",
        arguments: null,
      },
      {
        kind: "Linked",
        fieldName: "parent",
        arguments: null,
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
        ],
      },
      {
        kind: "Linked",
        fieldName: "pullRequests",
        arguments: [
          [
            "last",
            { kind: "Literal", value: null },
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
                    kind: "Linked",
                    fieldName: "author",
                    arguments: null,
                    selections: [
                      {
                        kind: "Scalar",
                        fieldName: "login",
                        arguments: null,
                      },
                    ],
                  },
                  {
                    kind: "Scalar",
                    fieldName: "closed",
                    arguments: null,
                  },
                  {
                    kind: "Scalar",
                    fieldName: "createdAt",
                    arguments: null,
                  },
                  {
                    kind: "Scalar",
                    fieldName: "number",
                    arguments: null,
                  },
                  {
                    kind: "Linked",
                    fieldName: "repository",
                    arguments: null,
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
                    ],
                  },
                  {
                    kind: "Scalar",
                    fieldName: "title",
                    arguments: null,
                  },
                  {
                    kind: "Scalar",
                    fieldName: "totalCommentsCount",
                    arguments: null,
                  },
                ],
              },
            ],
          },
        ],
      },
      {
        kind: "Scalar",
        fieldName: "stargazerCount",
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "viewerHasStarred",
        arguments: null,
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
  Query__RepositoryPage__param,
  Query__RepositoryPage__output_type
> = {
  kind: "Entrypoint",
  queryText,
  normalizationAst,
  nestedRefetchQueries,
  readerArtifact: readerResolver,
};

export default artifact;
