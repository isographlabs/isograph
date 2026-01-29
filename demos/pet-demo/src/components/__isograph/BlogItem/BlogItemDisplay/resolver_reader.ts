import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { BlogItem__BlogItemDisplay__param } from './param_type';
import { BlogItem as resolver } from '../../../Newsfeed/BlogItem';
import Image__ImageDisplayWrapper__resolver_reader from '../../Image/ImageDisplayWrapper/resolver_reader';

const readerAst: ReaderAst<BlogItem__BlogItemDisplay__param> = [
  {
    kind: "Scalar",
    isFallible: false,
    fieldName: "author",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Scalar",
    isFallible: false,
    fieldName: "title",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Scalar",
    isFallible: false,
    fieldName: "content",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "LoadablySelectedField",
    alias: "BlogItemMoreDetail",
    name: "BlogItemMoreDetail",
    queryArguments: null,
    refetchReaderAst: [
      {
        kind: "Scalar",
        isFallible: false,
        fieldName: "id",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
    ],
    entrypoint: {
      kind: "EntrypointLoader",
      typeAndField: "BlogItem__BlogItemMoreDetail",
      readerArtifactKind: "ComponentReaderArtifact",
      loader: () => import("../../BlogItem/BlogItemMoreDetail/entrypoint").then(module => module.default),
    },
  },
  {
    kind: "Linked",
    isFallible: true,
    fieldName: "image",
    alias: null,
    arguments: null,
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
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

const artifact = (): ComponentReaderArtifact<
  BlogItem__BlogItemDisplay__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "BlogItemDisplay",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
