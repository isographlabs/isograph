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
        { kind: "Variable", name: "repositoryOwner" },
      ],

      [
        "name",
        { kind: "Variable", name: "repositoryName" },
      ],
    ],
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Linked",
        fieldName: "pullRequest",
        alias: null,
        arguments: [
          [
            "number",
            { kind: "Variable", name: "pullRequestNumber" },
          ],
        ],
        condition: null,
        isUpdatable: false,
        refetchQueryIndex: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "title",
            alias: null,
            arguments: null,
            isUpdatable: false,
          },
          {
            kind: "Scalar",
            fieldName: "bodyHTML",
            alias: null,
            arguments: null,
            isUpdatable: false,
          },
          {
            kind: "Resolver",
            alias: "CommentList",
            arguments: [
              [
                "last",
                { kind: "Literal", value: 10 },
              ],
            ],
            readerArtifact: PullRequest__CommentList__resolver_reader,
            usedRefetchQueries: [],
          },
        ],
      },
    ],
  },
];

const artifact = (): ComponentReaderArtifact<
  Query__PullRequestDetail__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "PullRequestDetail",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
