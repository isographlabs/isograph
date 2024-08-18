import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Image__ImageDisplay__param } from './param_type';
import { ImageDisplay as resolver } from '../../../Newsfeed/ImageDisplay';

const readerAst: ReaderAst<Image__ImageDisplay__param> = [
  {
    kind: "Scalar",
    fieldName: "url",
    alias: null,
    arguments: null,
  },
];

const artifact: ComponentReaderArtifact<
  Image__ImageDisplay__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Image.ImageDisplay",
  resolver,
  readerAst,
};

export default artifact;
