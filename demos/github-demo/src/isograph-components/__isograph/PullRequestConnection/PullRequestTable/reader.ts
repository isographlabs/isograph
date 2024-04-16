import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { PullRequestConnection__PullRequestTable__param } from './param_type.ts';
import { PullRequestConnection__PullRequestTable__outputType } from './output_type.ts';
import { PullRequestTable as resolver } from '../../../PullRequestTable.tsx';
import Actor__UserLink from '../../Actor/UserLink/reader';
import PullRequest__PullRequestLink from '../../PullRequest/PullRequestLink/reader';
import PullRequest__createdAtFormatted from '../../PullRequest/createdAtFormatted/reader';

const readerAst: ReaderAst<PullRequestConnection__PullRequestTable__param> = [
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
            alias: "PullRequestLink",
            arguments: null,
            readerArtifact: PullRequest__PullRequestLink,
            usedRefetchQueries: [],
          },
          {
            kind: "Scalar",
            fieldName: "number",
            alias: null,
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "title",
            alias: null,
            arguments: null,
          },
          {
            kind: "Linked",
            fieldName: "author",
            alias: null,
            arguments: null,
            selections: [
              {
                kind: "Resolver",
                alias: "UserLink",
                arguments: null,
                readerArtifact: Actor__UserLink,
                usedRefetchQueries: [],
              },
              {
                kind: "Scalar",
                fieldName: "login",
                alias: null,
                arguments: null,
              },
            ],
          },
          {
            kind: "Scalar",
            fieldName: "closed",
            alias: null,
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "totalCommentsCount",
            alias: null,
            arguments: null,
          },
          {
            kind: "Resolver",
            alias: "createdAtFormatted",
            arguments: null,
            readerArtifact: PullRequest__createdAtFormatted,
            usedRefetchQueries: [],
          },
        ],
      },
    ],
  },
];

const artifact: ReaderArtifact<
  PullRequestConnection__PullRequestTable__param,
  PullRequestConnection__PullRequestTable__outputType
> = {
  kind: "ReaderArtifact",
  fieldName: "PullRequestTable",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "PullRequestConnection.PullRequestTable" },
};

export default artifact;
