import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { AdItem__AdItemDisplay__param } from './param_type';
import { BlogItem as resolver } from '../../../Newsfeed/AdItem';

const readerAst: ReaderAst<AdItem__AdItemDisplay__param> = [
  {
    kind: "Scalar",
    fieldName: "advertiser",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Scalar",
    fieldName: "message",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
];

const artifact: ComponentReaderArtifact<
  AdItem__AdItemDisplay__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "AdItem.AdItemDisplay",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
