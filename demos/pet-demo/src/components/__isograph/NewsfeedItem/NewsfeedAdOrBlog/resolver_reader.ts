import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { NewsfeedItem__NewsfeedAdOrBlog__param } from './param_type';
import { NewsfeedAdOrBlog as resolver } from '../../../Newsfeed/NewsfeedRoute';
import AdItem__AdItemDisplayWrapper__resolver_reader from '../../AdItem/AdItemDisplayWrapper/resolver_reader';
import AdItem__asAdItem__resolver_reader from '../../AdItem/asAdItem/resolver_reader';
import BlogItem__BlogItemDisplay__resolver_reader from '../../BlogItem/BlogItemDisplay/resolver_reader';
import BlogItem__asBlogItem__resolver_reader from '../../BlogItem/asBlogItem/resolver_reader';

const readerAst: ReaderAst<NewsfeedItem__NewsfeedAdOrBlog__param> = [
  {
    kind: "Linked",
    fieldName: "asAdItem",
    alias: null,
    arguments: null,
    condition: AdItem__asAdItem__resolver_reader,
    selections: [
      {
        kind: "Resolver",
        alias: "AdItemDisplayWrapper",
        arguments: null,
        readerArtifact: AdItem__AdItemDisplayWrapper__resolver_reader,
        usedRefetchQueries: [],
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
        kind: "Resolver",
        alias: "BlogItemDisplay",
        arguments: null,
        readerArtifact: BlogItem__BlogItemDisplay__resolver_reader,
        usedRefetchQueries: [],
      },
    ],
  },
];

const artifact: ComponentReaderArtifact<
  NewsfeedItem__NewsfeedAdOrBlog__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "NewsfeedItem.NewsfeedAdOrBlog",
  resolver,
  readerAst,
};

export default artifact;
