import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Viewer__NewsfeedPaginationComponent__param } from './param_type';
import { Viewer__NewsfeedPaginationComponent__output_type } from './output_type';
import { NewsfeedPaginationComponent as resolver } from '../../../Newsfeed/NewsfeedPagination';
import AdItem__asAdItem__resolver_reader from '../../AdItem/asAdItem/resolver_reader';
import BlogItem__asBlogItem__resolver_reader from '../../BlogItem/asBlogItem/resolver_reader';
import NewsfeedItem__NewsfeedAdOrBlog__resolver_reader from '../../NewsfeedItem/NewsfeedAdOrBlog/resolver_reader';

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
    selections: [
      {
        kind: "Linked",
        fieldName: "asAdItem",
        alias: null,
        arguments: null,
        condition: AdItem__asAdItem__resolver_reader,
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            alias: null,
            arguments: null,
          },
        ],
      },
      {
        kind: "Linked",
        fieldName: "asBlogItem",
        alias: null,
        arguments: null,
        condition: BlogItem__asBlogItem__resolver_reader,
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            alias: null,
            arguments: null,
          },
        ],
      },
      {
        kind: "Resolver",
        alias: "NewsfeedAdOrBlog",
        arguments: null,
        readerArtifact: NewsfeedItem__NewsfeedAdOrBlog__resolver_reader,
        usedRefetchQueries: [],
      },
    ],
  },
];

const artifact: EagerReaderArtifact<
  Viewer__NewsfeedPaginationComponent__param,
  Viewer__NewsfeedPaginationComponent__output_type
> = {
  kind: "EagerReaderArtifact",
  resolver,
  readerAst,
};

export default artifact;
