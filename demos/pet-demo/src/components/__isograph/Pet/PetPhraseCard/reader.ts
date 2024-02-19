import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { PetPhraseCard as resolver } from '../../../PetPhraseCard.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (React.FC<any>);

export type ReadFromStoreType = Pet__PetPhraseCard__param;

const readerAst: ReaderAst<ReadFromStoreType> = [
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

const artifact: ReaderArtifact<ReadFromStoreType, Pet__PetPhraseCard__param, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Pet.PetPhraseCard" },
};

export default artifact;
