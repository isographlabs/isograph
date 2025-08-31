import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Pet__FavoritePhraseLoader__param } from './param_type';
import { FavoritePhraseLoader as resolver } from '../../../Pet/FavoritePhraseLoader';

const readerAst: ReaderAst<Pet__FavoritePhraseLoader__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
];

const artifact: ComponentReaderArtifact<
  Pet__FavoritePhraseLoader__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "Pet.FavoritePhraseLoader",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
