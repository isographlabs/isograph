import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { NewsfeedItem__NewsfeedAdOrBlog__param } from './param_type';
import { NewsfeedAdOrBlog as resolver } from '../../../Newsfeed/NewsfeedRoute';
import BlogItem__BlogItemDisplay__resolver_reader from '../../BlogItem/BlogItemDisplay/resolver_reader';
import NewsfeedItem__asAdItem__resolver_reader from '../../NewsfeedItem/asAdItem/resolver_reader';
import NewsfeedItem__asBlogItem__resolver_reader from '../../NewsfeedItem/asBlogItem/resolver_reader';

const readerAst: ReaderAst<NewsfeedItem__NewsfeedAdOrBlog__param> = [
  {
    kind: "Linked",
    fieldName: "asAdItem",
    alias: null,
    arguments: null,
    condition: NewsfeedItem__asAdItem__resolver_reader,
    isUpdatable: false,
    selections: [
      {
        kind: "LoadablySelectedField",
        alias: "AdItemDisplay",
        name: "AdItemDisplay",
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
        entrypoint: { 
          kind: "EntrypointLoader",
          typeAndField: "AdItem__AdItemDisplay",
          loader: () => import("../../AdItem/AdItemDisplay/entrypoint").then(module => module.default),
        },
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
        kind: "Resolver",
        alias: "BlogItemDisplay",
        arguments: null,
        readerArtifact: BlogItem__BlogItemDisplay__resolver_reader,
        usedRefetchQueries: [],
      },
    ],
    refetchQueryIndex: null,
  },
];

const artifact: ComponentReaderArtifact<
  NewsfeedItem__NewsfeedAdOrBlog__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "NewsfeedItem.NewsfeedAdOrBlog",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
