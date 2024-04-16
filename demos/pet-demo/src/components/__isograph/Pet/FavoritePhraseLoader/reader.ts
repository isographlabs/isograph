import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Pet__FavoritePhraseLoader__param } from './param_type.ts';
import { Pet__FavoritePhraseLoader__outputType } from './output_type.ts';
import { FavoritePhraseLoader as resolver } from '../../../FavoritePhrase.tsx';

const readerAst: ReaderAst<Pet__FavoritePhraseLoader__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
];

const artifact: ReaderArtifact<
  Pet__FavoritePhraseLoader__param,
  Pet__FavoritePhraseLoader__outputType
> = {
  kind: "ReaderArtifact",
  fieldName: "FavoritePhraseLoader",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Pet.FavoritePhraseLoader" },
};

export default artifact;
