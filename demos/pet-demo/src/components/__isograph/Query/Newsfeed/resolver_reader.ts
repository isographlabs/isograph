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
    isUpdatable: false,
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
            isUpdatable: false,
          },
        ],
        entrypoint: Viewer__NewsfeedPaginationComponent__entrypoint,
      },
    ],
    refetchQueryIndex: null,
  },
];

const artifact: ComponentReaderArtifact<
  Query__Newsfeed__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "Query.Newsfeed",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
