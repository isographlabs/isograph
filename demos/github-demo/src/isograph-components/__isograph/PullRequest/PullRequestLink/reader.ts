import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { PullRequest__PullRequestLink__param } from './param_type.ts';
import { PullRequest__PullRequestLink__outputType } from './output_type.ts';
import { PullRequestLink as resolver } from '../../../PullRequestLink.tsx';

const readerAst: ReaderAst<PullRequest__PullRequestLink__param> = [
  {
    kind: "Scalar",
    fieldName: "number",
    alias: null,
    arguments: null,
  },
  {
    kind: "Linked",
    fieldName: "repository",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
      },
      {
        kind: "Linked",
        fieldName: "owner",
        alias: null,
        arguments: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "login",
            alias: null,
            arguments: null,
          },
        ],
      },
    ],
  },
];

const artifact: ReaderArtifact<
  PullRequest__PullRequestLink__param,
  PullRequest__PullRequestLink__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "PullRequest.PullRequestLink" },
};

export default artifact;
