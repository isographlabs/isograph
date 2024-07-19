import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { PullRequestConnection__PullRequestTable__param } from './param_type';
import { PullRequestTable as resolver } from '../../../PullRequestTable';
import Actor__UserLink__resolver_reader from '../../Actor/UserLink/resolver_reader';
import PullRequest__PullRequestLink__resolver_reader from '../../PullRequest/PullRequestLink/resolver_reader';
import PullRequest__createdAtFormatted__resolver_reader from '../../PullRequest/createdAtFormatted/resolver_reader';

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
            readerArtifact: PullRequest__PullRequestLink__resolver_reader,
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
                readerArtifact: Actor__UserLink__resolver_reader,
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
            readerArtifact: PullRequest__createdAtFormatted__resolver_reader,
            usedRefetchQueries: [],
          },
        ],
      },
    ],
  },
];

const artifact: ComponentReaderArtifact<
  PullRequestConnection__PullRequestTable__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "PullRequestConnection.PullRequestTable",
  resolver,
  readerAst,
};

export default artifact;
