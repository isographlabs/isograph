import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Viewer__NewsfeedPaginationComponent__param } from './param_type';
import { Viewer__NewsfeedPaginationComponent__output_type } from './output_type';
import { NewsfeedPaginationComponent as resolver } from '../../../Newsfeed/NewsfeedPagination';
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

      [
        "additionalSkip",
        { kind: "Literal", value: 5 },
      ],
    ],
    selections: [
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
