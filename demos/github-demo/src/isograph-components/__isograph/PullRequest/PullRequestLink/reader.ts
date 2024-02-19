import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { PullRequestLink as resolver } from '../../../PullRequestLink.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type PullRequest__PullRequestLink__outputType = (React.FC<any>);

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

export type PullRequest__PullRequestLink__param = { data:
{
  number: number,
  repository: {
    name: string,
    owner: {
      login: string,
    },
  },
},
[index: string]: any };

const artifact: ReaderArtifact<
  PullRequest__PullRequestLink__param,
  PullRequest__PullRequestLink__param,
  PullRequest__PullRequestLink__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "PullRequest.PullRequestLink" },
};

export default artifact;
