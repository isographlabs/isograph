import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__Newsfeed__param } from './param_type';
import { Newsfeed as resolver } from '../../../Newsfeed/NewsfeedRoute';
import Viewer__NewsfeedPaginationComponent__resolver_reader from '../../Viewer/NewsfeedPaginationComponent/resolver_reader';
import Viewer__NewsfeedPaginationComponent__entrypoint from '../../Viewer/NewsfeedPaginationComponent/entrypoint';

const readerAst: ReaderAst<Query__Newsfeed__param> = [
  {
    kind: "Linked",
    fieldName: "viewer",
    alias: null,
    arguments: null,
    condition: null,
    selections: [
      {
        kind: "Resolver",
        alias: "initial",
        arguments: [
          [
            "skip",
            { kind: "Literal", value: 0 },
          ],

          [
            "limit",
            { kind: "Literal", value: 6 },
          ],
        ],
        readerArtifact: Viewer__NewsfeedPaginationComponent__resolver_reader,
        usedRefetchQueries: [],
      },
      {
        kind: "LoadablySelectedField",
        alias: "NewsfeedPaginationComponent",
        name: "NewsfeedPaginationComponent",
        queryArguments: null,
        refetchReaderAst: [
          {
            kind: "Scalar",
            fieldName: "id",
            alias: null,
            arguments: null,
          },
        ],
        entrypoint: Viewer__NewsfeedPaginationComponent__entrypoint,
      },
    ],
  },
];

const artifact: ComponentReaderArtifact<
  Query__Newsfeed__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Query.Newsfeed",
  resolver,
  readerAst,
};

export default artifact;
