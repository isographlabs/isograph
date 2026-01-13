import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { PullRequest__PullRequestLink__param } from './param_type';
import { PullRequestLink as resolver } from '../../../PullRequestLink';

const readerAst: ReaderAst<PullRequest__PullRequestLink__param> = [
  {
    kind: "Scalar",
    fieldName: "number",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Linked",
    fieldName: "repository",
    alias: null,
    arguments: null,
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Linked",
        fieldName: "owner",
        alias: null,
        arguments: null,
        condition: null,
        isUpdatable: false,
        refetchQueryIndex: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "login",
            alias: null,
            arguments: null,
            isUpdatable: false,
          },
        ],
      },
    ],
  },
];

const artifact = (): ComponentReaderArtifact<
  PullRequest__PullRequestLink__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "PullRequestLink",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
