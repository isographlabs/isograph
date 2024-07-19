import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Pet__PetSummaryCard__param } from './param_type';
import { PetSummaryCard as resolver } from '../../../PetSummaryCard';
import Pet__FavoritePhraseLoader__resolver_reader from '../../Pet/FavoritePhraseLoader/resolver_reader';

const readerAst: ReaderAst<Pet__PetSummaryCard__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "name",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "picture",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "tagline",
    alias: null,
    arguments: null,
  },
  {
    kind: "Resolver",
    alias: "FavoritePhraseLoader",
    arguments: null,
    readerArtifact: Pet__FavoritePhraseLoader__resolver_reader,
    usedRefetchQueries: [],
  },
];

const artifact: ComponentReaderArtifact<
  Pet__PetSummaryCard__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Pet.PetSummaryCard",
  resolver,
  readerAst,
};

export default artifact;
