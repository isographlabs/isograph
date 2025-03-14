import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { BlogItem__BlogItemMoreDetail__param } from './param_type';
import { BlogItemMoreDetail as resolver } from '../../../Newsfeed/BlogItemMoreDetail';

const readerAst: ReaderAst<BlogItem__BlogItemMoreDetail__param> = [
  {
    kind: "Scalar",
    fieldName: "moreContent",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
];

const artifact: ComponentReaderArtifact<
  BlogItem__BlogItemMoreDetail__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "BlogItem.BlogItemMoreDetail",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
