import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { User__RepositoryConnection__param } from './param_type';
import { User__RepositoryConnection__output_type } from './output_type';
import { RepositoryConnection as resolver } from '../../../UserRepositoryList';
import Repository__RepositoryRow__resolver_reader from '../../Repository/RepositoryRow/resolver_reader';

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
    condition: null,
    selections: [
      {
        kind: "Linked",
        fieldName: "pageInfo",
        alias: null,
        arguments: null,
        condition: null,
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
        condition: null,
        selections: [
          {
            kind: "Linked",
            fieldName: "node",
            alias: null,
            arguments: null,
            condition: null,
            selections: [
              {
                kind: "Resolver",
                alias: "RepositoryRow",
                arguments: null,
                readerArtifact: Repository__RepositoryRow__resolver_reader,
                usedRefetchQueries: [],
              },
              {
                kind: "Scalar",
                fieldName: "id",
                alias: null,
                arguments: null,
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
  fieldName: "User.RepositoryConnection",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
