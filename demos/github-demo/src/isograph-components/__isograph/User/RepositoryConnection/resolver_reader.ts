import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { User__RepositoryConnection__param } from './param_type';
import { User__RepositoryConnection__output_type } from './output_type';
import { RepositoryConnection as resolver } from '../../../UserRepositoryList';
import Repository__RepositoryLink__resolver_reader from '../../Repository/RepositoryLink/resolver_reader';

const readerAst: ReaderAst<User__RepositoryConnection__param> = [
  {
    kind: "Linked",
    fieldName: "repositories",
    alias: null,
    arguments: [
      [
        "first",
        { kind: "Variable", name: "first" },
      ],

      [
        "after",
        { kind: "Variable", name: "after" },
      ],
    ],
    selections: [
      {
        kind: "Linked",
        fieldName: "pageInfo",
        alias: null,
        arguments: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "hasNextPage",
            alias: null,
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "endCursor",
            alias: null,
            arguments: null,
          },
        ],
      },
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
                readerArtifact: Repository__RepositoryLink__resolver_reader,
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
                arguments: null,
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
                arguments: null,
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

const artifact: EagerReaderArtifact<
  User__RepositoryConnection__param,
  User__RepositoryConnection__output_type
> = {
  kind: "EagerReaderArtifact",
  resolver,
  readerAst,
};

export default artifact;
