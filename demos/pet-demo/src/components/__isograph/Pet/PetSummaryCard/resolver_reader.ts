import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Pet__PetSummaryCard__param } from './param_type';
import { PetSummaryCard as resolver } from '../../../Pet/PetSummaryCard';
import Pet__Avatar__resolver_reader from '../../Pet/Avatar/resolver_reader';
import Pet__FavoritePhraseLoader__resolver_reader from '../../Pet/FavoritePhraseLoader/resolver_reader';

const readerAst: ReaderAst<Pet__PetSummaryCard__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Scalar",
    fieldName: "name",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Resolver",
    alias: "Avatar",
    arguments: null,
    readerArtifact: Pet__Avatar__resolver_reader,
    usedRefetchQueries: [],
  },
  {
    kind: "Scalar",
    fieldName: "tagline",
    alias: null,
    arguments: null,
    isUpdatable: false,
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
  fieldName: "Pet.PetSummaryCard",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
