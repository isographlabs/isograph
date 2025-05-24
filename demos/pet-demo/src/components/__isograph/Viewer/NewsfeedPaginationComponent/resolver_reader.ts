import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Viewer__NewsfeedPaginationComponent__param } from './param_type';
import { Viewer__NewsfeedPaginationComponent__output_type } from './output_type';
import { NewsfeedPaginationComponent as resolver } from '../../../Newsfeed/NewsfeedPagination';
import NewsfeedItem__NewsfeedAdOrBlog__resolver_reader from '../../NewsfeedItem/NewsfeedAdOrBlog/resolver_reader';
import NewsfeedItem__asAdItem__resolver_reader from '../../NewsfeedItem/asAdItem/resolver_reader';
import NewsfeedItem__asBlogItem__resolver_reader from '../../NewsfeedItem/asBlogItem/resolver_reader';

const readerAst: ReaderAst<Viewer__NewsfeedPaginationComponent__param> = [
  {
    kind: "Linked",
    fieldName: "newsfeed",
    alias: null,
    arguments: [
      [
        "skip",
        { kind: "Variable", name: "skip" },
      ],

      [
        "limit",
        { kind: "Variable", name: "limit" },
      ],
    ],
    condition: null,
    isUpdatable: false,
    selections: [
      {
        kind: "Linked",
        fieldName: "asAdItem",
        alias: null,
        arguments: null,
        condition: NewsfeedItem__asAdItem__resolver_reader,
        isUpdatable: false,
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            alias: null,
            arguments: null,
            isUpdatable: false,
          },
        ],
        refetchQueryIndex: null,
      },
      {
        kind: "Linked",
        fieldName: "asBlogItem",
        alias: null,
        arguments: null,
        condition: NewsfeedItem__asBlogItem__resolver_reader,
        isUpdatable: false,
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            alias: null,
            arguments: null,
            isUpdatable: false,
          },
        ],
        refetchQueryIndex: null,
      },
      {
        kind: "Resolver",
        alias: "NewsfeedAdOrBlog",
        arguments: null,
        readerArtifact: NewsfeedItem__NewsfeedAdOrBlog__resolver_reader,
        usedRefetchQueries: [],
      },
    ],
    refetchQueryIndex: null,
  },
];

const artifact: EagerReaderArtifact<
  Viewer__NewsfeedPaginationComponent__param,
  Viewer__NewsfeedPaginationComponent__output_type
> = {
  kind: "EagerReaderArtifact",
  fieldName: "Viewer.NewsfeedPaginationComponent",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
