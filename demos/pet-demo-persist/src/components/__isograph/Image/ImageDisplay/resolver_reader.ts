import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Image__ImageDisplay__param } from './param_type';
import { ImageDisplay as resolver } from '../../../Newsfeed/ImageDisplay';

const readerAst: ReaderAst<Image__ImageDisplay__param> = [
  {
    kind: "Scalar",
    fieldName: "url",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
];

const artifact: ComponentReaderArtifact<
  Image__ImageDisplay__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "Image.ImageDisplay",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
