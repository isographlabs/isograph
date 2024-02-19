import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { PetPhraseCard as resolver } from '../../../PetPhraseCard.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type Pet__PetPhraseCard__outputType = (React.FC<any>);

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

export type Pet__PetPhraseCard__param = { data:
{
  id: string,
  favorite_phrase: (string | null),
},
[index: string]: any };

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
