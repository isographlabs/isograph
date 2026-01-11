import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { PullRequestConnection__PullRequestTable__param } from './param_type';
import { PullRequestTable as resolver } from '../../../PullRequestTable';
import Actor__UserLink__resolver_reader from '../../Actor/UserLink/resolver_reader';
import PullRequest__PullRequestLink__resolver_reader from '../../PullRequest/PullRequestLink/resolver_reader';
import PullRequest__createdAtFormatted__resolver_reader from '../../PullRequest/createdAtFormatted/resolver_reader';

const readerAst: ReaderAst<PullRequestConnection__PullRequestTable__param> = [
  {
    kind: "Linked",
    isFallible: true,
    fieldName: "edges",
    alias: null,
    arguments: null,
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Linked",
        isFallible: true,
        fieldName: "node",
        alias: null,
        arguments: null,
        condition: null,
        isUpdatable: false,
        refetchQueryIndex: null,
        selections: [
          {
            kind: "Scalar",
            isFallible: false,
            fieldName: "id",
            alias: null,
            arguments: null,
            isUpdatable: false,
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
            isFallible: false,
            fieldName: "number",
            alias: null,
            arguments: null,
            isUpdatable: false,
          },
          {
            kind: "Scalar",
            isFallible: false,
            fieldName: "title",
            alias: null,
            arguments: null,
            isUpdatable: false,
          },
          {
            kind: "Linked",
            isFallible: true,
            fieldName: "author",
            alias: null,
            arguments: null,
            condition: null,
            isUpdatable: false,
            refetchQueryIndex: null,
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
                isFallible: false,
                fieldName: "login",
                alias: null,
                arguments: null,
                isUpdatable: false,
              },
            ],
          },
          {
            kind: "Scalar",
            isFallible: false,
            fieldName: "closed",
            alias: null,
            arguments: null,
            isUpdatable: false,
          },
          {
            kind: "Scalar",
            isFallible: true,
            fieldName: "totalCommentsCount",
            alias: null,
            arguments: null,
            isUpdatable: false,
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

const artifact = (): ComponentReaderArtifact<
  PullRequestConnection__PullRequestTable__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "PullRequestTable",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
