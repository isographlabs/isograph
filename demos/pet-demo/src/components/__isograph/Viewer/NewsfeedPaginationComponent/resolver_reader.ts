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
    isFallible: false,
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
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Scalar",
        isFallible: false,
        fieldName: "__typename",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Linked",
        isFallible: false,
        fieldName: "asAdItem",
        alias: null,
        arguments: null,
        condition: NewsfeedItem__asAdItem__resolver_reader,
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
        ],
      },
      {
        kind: "Linked",
        isFallible: false,
        fieldName: "asBlogItem",
        alias: null,
        arguments: null,
        condition: NewsfeedItem__asBlogItem__resolver_reader,
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

const artifact = (): EagerReaderArtifact<
  Viewer__NewsfeedPaginationComponent__param,
  Viewer__NewsfeedPaginationComponent__output_type
> => ({
  kind: "EagerReaderArtifact",
  fieldName: "NewsfeedPaginationComponent",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
