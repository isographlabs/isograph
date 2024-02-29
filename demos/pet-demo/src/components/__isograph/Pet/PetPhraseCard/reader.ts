import type {ReaderArtifact, ReaderAst, ExtractSecondParam} from '@isograph/react';
import { PetPhraseCard as resolver } from '../../../PetPhraseCard.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type Pet__PetPhraseCard__outputType = (React.FC<ExtractSecondParam<typeof resolver>>);

const readerAst: ReaderAst<Pet__PetPhraseCard__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "favorite_phrase",
    alias: null,
    arguments: null,
  },
];

export type Pet__PetPhraseCard__param = {
  id: string,
  favorite_phrase: (string | null),
};

const artifact: ReaderArtifact<
  Pet__PetPhraseCard__param,
  Pet__PetPhraseCard__param,
  Pet__PetPhraseCard__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Pet.PetPhraseCard" },
};

export default artifact;
