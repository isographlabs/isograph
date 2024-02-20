import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { FavoritePhraseLoader as resolver } from '../../../FavoritePhrase.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type Pet__FavoritePhraseLoader__outputType = (React.FC<any>);

const readerAst: ReaderAst<Pet__FavoritePhraseLoader__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
];

export type Pet__FavoritePhraseLoader__param = { data:
{
  id: string,
},
[index: string]: any };

const artifact: ReaderArtifact<
  Pet__FavoritePhraseLoader__param,
  Pet__FavoritePhraseLoader__param,
  Pet__FavoritePhraseLoader__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Pet.FavoritePhraseLoader" },
};

export default artifact;
