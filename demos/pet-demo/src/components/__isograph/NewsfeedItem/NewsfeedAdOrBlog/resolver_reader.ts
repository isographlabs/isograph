import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { NewsfeedItem__NewsfeedAdOrBlog__param } from './param_type';
import { NewsfeedAdOrBlog as resolver } from '../../../Newsfeed/NewsfeedRoute';
import AdItem__AdItemDisplayWrapper__resolver_reader from '../../AdItem/AdItemDisplayWrapper/resolver_reader';
import BlogItem__BlogItemDisplay__resolver_reader from '../../BlogItem/BlogItemDisplay/resolver_reader';

const readerAst: ReaderAst<NewsfeedItem__NewsfeedAdOrBlog__param> = [
  {
    kind: "Linked",
    fieldName: "adItem",
    alias: null,
    arguments: null,
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
    fieldName: "blogItem",
    alias: null,
    arguments: null,
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
