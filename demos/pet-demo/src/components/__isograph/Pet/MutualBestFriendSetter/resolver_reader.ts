import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Pet__MutualBestFriendSetter__param } from './param_type';
import { MutualBestFriendSetter as resolver } from '../../../Pet/MutualBestFriendSetter';

const readerAst: ReaderAst<Pet__MutualBestFriendSetter__param> = [
  {
    kind: "Scalar",
    isFallible: false,
    fieldName: "id",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
];

const artifact = (): ComponentReaderArtifact<
  Pet__MutualBestFriendSetter__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "MutualBestFriendSetter",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
