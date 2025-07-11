import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { PullRequest__CommentList__param } from './param_type';
import { CommentList as resolver } from '../../../CommentList';
import IssueComment__formattedCommentCreationDate__resolver_reader from '../../IssueComment/formattedCommentCreationDate/resolver_reader';

const readerAst: ReaderAst<PullRequest__CommentList__param> = [
  {
    kind: "Linked",
    fieldName: "comments",
    alias: null,
    arguments: [
      [
        "last",
        { kind: "Variable", name: "last" },
      ],
    ],
    condition: null,
    isUpdatable: false,
    selections: [
      {
        kind: "Linked",
        fieldName: "edges",
        alias: null,
        arguments: null,
        condition: null,
        isUpdatable: false,
        selections: [
          {
            kind: "Linked",
            fieldName: "node",
            alias: null,
            arguments: null,
            condition: null,
            isUpdatable: false,
            selections: [
              {
                kind: "Scalar",
                fieldName: "id",
                alias: null,
                arguments: null,
                isUpdatable: false,
              },
              {
                kind: "Scalar",
                fieldName: "bodyText",
                alias: null,
                arguments: null,
                isUpdatable: false,
              },
              {
                kind: "Resolver",
                alias: "formattedCommentCreationDate",
                arguments: null,
                readerArtifact: IssueComment__formattedCommentCreationDate__resolver_reader,
                usedRefetchQueries: [],
              },
              {
                kind: "Linked",
                fieldName: "author",
                alias: null,
                arguments: null,
                condition: null,
                isUpdatable: false,
                selections: [
                  {
                    kind: "Scalar",
                    fieldName: "login",
                    alias: null,
                    arguments: null,
                    isUpdatable: false,
                  },
                ],
                refetchQueryIndex: null,
              },
            ],
            refetchQueryIndex: null,
          },
        ],
        refetchQueryIndex: null,
      },
    ],
    refetchQueryIndex: null,
  },
];

const artifact: ComponentReaderArtifact<
  PullRequest__CommentList__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "PullRequest.CommentList",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
