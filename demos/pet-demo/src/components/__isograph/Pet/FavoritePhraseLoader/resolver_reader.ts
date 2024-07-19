import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Pet__FavoritePhraseLoader__param } from './param_type';
import { FavoritePhraseLoader as resolver } from '../../../FavoritePhrase';

const readerAst: ReaderAst<Pet__FavoritePhraseLoader__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
];

const artifact: ComponentReaderArtifact<
  Pet__FavoritePhraseLoader__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Pet.FavoritePhraseLoader",
  resolver,
  readerAst,
};

export default artifact;
