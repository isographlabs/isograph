import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Pet__Avatar__param } from './param_type';
import { PetAvatar as resolver } from '../../../Pet/Avatar';

const readerAst: ReaderAst<Pet__Avatar__param> = [
  {
    kind: "Scalar",
    fieldName: "picture",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
];

const artifact: ComponentReaderArtifact<
  Pet__Avatar__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "Pet.Avatar",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
