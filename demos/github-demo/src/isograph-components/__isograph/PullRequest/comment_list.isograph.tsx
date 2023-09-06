import type {IsographNonFetchableResolver, ReaderAst} from '@isograph/react';
import { comment_list as resolver } from '../../comment_list.tsx';
import IssueComment__formatted_comment_creation_date, { ReadOutType as IssueComment__formatted_comment_creation_date__outputType } from '../IssueComment/formatted_comment_creation_date.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (additionalRuntimeProps: Object | void) => (React.ReactElement<any, any> | null);

// TODO support changing this
export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Linked",
    fieldName: "comments",
    alias: null,
    arguments: [
      {
        argumentName: "last",
        variableName: "last",
      },
    ],
    selections: [
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
                kind: "Scalar",
                fieldName: "bodyText",
                alias: null,
                arguments: null,
              },
              {
                kind: "Resolver",
                alias: "formatted_comment_creation_date",
                arguments: null,
                resolver: IssueComment__formatted_comment_creation_date,
                variant: "Eager",
                usedRefetchQueries: [0, 1, ],
              },
              {
                kind: "Linked",
                fieldName: "author",
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
        ],
      },
    ],
  },
];

export type ResolverParameterType = { data:
{
  comments: {
    edges: (({
      node: ({
        id: string,
        bodyText: string,
        formatted_comment_creation_date: IssueComment__formatted_comment_creation_date__outputType,
        author: ({
          login: string,
        } | null),
      } | null),
    } | null))[],
  },
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: IsographNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'NonFetchableResolver',
  resolver: resolver as any,
  readerAst,
};

export default artifact;
