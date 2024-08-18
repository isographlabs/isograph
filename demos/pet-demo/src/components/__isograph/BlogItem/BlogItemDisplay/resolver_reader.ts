import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { BlogItem__BlogItemDisplay__param } from './param_type';
import { BlogItem as resolver } from '../../../Newsfeed/BlogItem';
import Image__ImageDisplayWrapper__resolver_reader from '../../Image/ImageDisplayWrapper/resolver_reader';

const readerAst: ReaderAst<BlogItem__BlogItemDisplay__param> = [
  {
    kind: "Scalar",
    fieldName: "author",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "title",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "content",
    alias: null,
    arguments: null,
  },
  {
    kind: "LoadablySelectedField",
    alias: "BlogItemMoreDetail",
    name: "BlogItemMoreDetail",
    queryArguments: null,
    refetchReaderAst: [
      {
        kind: "Scalar",
        fieldName: "id",
        alias: null,
        arguments: null,
      },
    ],
    entrypoint: { 
      kind: "EntrypointLoader",
      typeAndField: "BlogItem__BlogItemMoreDetail",
      loader: () => import("../../BlogItem/BlogItemMoreDetail/entrypoint").then(module => module.default),
    },
  },
  {
    kind: "Linked",
    fieldName: "image",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Resolver",
        alias: "ImageDisplayWrapper",
        arguments: null,
        readerArtifact: Image__ImageDisplayWrapper__resolver_reader,
        usedRefetchQueries: [],
      },
    ],
  },
];

const artifact: ComponentReaderArtifact<
  BlogItem__BlogItemDisplay__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "BlogItem.BlogItemDisplay",
  resolver,
  readerAst,
};

export default artifact;
