import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { PullRequest__CommentList__param } from './param_type';
import { CommentList as resolver } from '../../../CommentList';
import IssueComment__formattedCommentCreationDate__resolver_reader from '../../IssueComment/formattedCommentCreationDate/resolver_reader';

const readerAst: ReaderAst<PullRequest__CommentList__param> = [
  {
    kind: "Linked",
    isFallible: false,
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
    refetchQueryIndex: null,
    selections: [
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
                kind: "Scalar",
                isFallible: false,
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
                isFallible: true,
                fieldName: "author",
                alias: null,
                arguments: null,
                condition: null,
                isUpdatable: false,
                refetchQueryIndex: null,
                selections: [
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
            ],
          },
        ],
      },
    ],
  },
];

const artifact = (): ComponentReaderArtifact<
  PullRequest__CommentList__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "CommentList",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
