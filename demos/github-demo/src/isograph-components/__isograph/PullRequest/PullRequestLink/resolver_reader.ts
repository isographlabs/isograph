import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { PullRequest__PullRequestLink__param } from './param_type';
import { PullRequestLink as resolver } from '../../../PullRequestLink';
import User__asUser__resolver_reader from '../../User/asUser/resolver_reader';

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
    condition: null,
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
        condition: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "login",
            alias: null,
            arguments: null,
          },
          {
            kind: "Linked",
            fieldName: "asUser",
            alias: null,
            arguments: null,
            condition: User__asUser__resolver_reader,
            selections: [
              {
                kind: "Scalar",
                fieldName: "id",
                alias: null,
                arguments: null,
              },
              {
                kind: "Scalar",
                fieldName: "bio",
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

const artifact: ComponentReaderArtifact<
  PullRequest__PullRequestLink__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "PullRequest.PullRequestLink",
  resolver,
  readerAst,
};

export default artifact;
