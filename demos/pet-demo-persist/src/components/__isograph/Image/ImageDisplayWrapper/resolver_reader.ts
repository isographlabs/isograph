import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Image__ImageDisplayWrapper__param } from './param_type';
import { ImageDisplayWrapper as resolver } from '../../../Newsfeed/BlogItem';

const readerAst: ReaderAst<Image__ImageDisplayWrapper__param> = [
  {
    kind: "LoadablySelectedField",
    alias: "ImageDisplay",
    name: "ImageDisplay",
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
      typeAndField: "Image__ImageDisplay",
      loader: () => import("../../Image/ImageDisplay/entrypoint").then(module => module.default),
    },
  },
];

const artifact: ComponentReaderArtifact<
  Image__ImageDisplayWrapper__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "Image.ImageDisplayWrapper",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
