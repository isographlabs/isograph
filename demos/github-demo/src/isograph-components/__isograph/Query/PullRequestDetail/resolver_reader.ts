import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__PullRequestDetail__param } from './param_type';
import { PullRequestDetail as resolver } from '../../../PullRequestDetail';
import PullRequest__CommentList__resolver_reader from '../../PullRequest/CommentList/resolver_reader';

const readerAst: ReaderAst<Query__PullRequestDetail__param> = [
  {
    kind: "Linked",
    fieldName: "repository",
    alias: null,
    arguments: [
      [
        "owner",
        { kind: "Literal", value: null },
      ],

      [
        "name",
        { kind: "Literal", value: null },
      ],
    ],
    selections: [
      {
        kind: "Linked",
        fieldName: "pullRequest",
        alias: null,
        arguments: [
          [
            "number",
            { kind: "Literal", value: null },
          ],
        ],
        selections: [
          {
            kind: "Scalar",
            fieldName: "title",
            alias: null,
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "bodyHTML",
            alias: null,
            arguments: null,
          },
          {
            kind: "Resolver",
            alias: "CommentList",
            arguments: null,
            readerArtifact: PullRequest__CommentList__resolver_reader,
            usedRefetchQueries: [],
          },
        ],
      },
    ],
  },
];

const artifact: ComponentReaderArtifact<
  Query__PullRequestDetail__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Query.PullRequestDetail",
  resolver,
  readerAst,
};

export default artifact;
