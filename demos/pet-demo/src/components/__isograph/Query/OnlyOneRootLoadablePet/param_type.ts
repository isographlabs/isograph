import { type Query__PetFavoritePhrase__output_type } from '../../Query/PetFavoritePhrase/output_type';
import { type LoadableField, type ExtractParameters } from '@isograph/react';
import { type Query__PetFavoritePhrase__param } from '../../Query/PetFavoritePhrase/param_type';
import type { Query__OnlyOneRootLoadablePet__parameters } from './parameters_type';

export type Query__OnlyOneRootLoadablePet__param = {
  readonly data: {
    readonly PetFavoritePhrase: LoadableField<
      Query__PetFavoritePhrase__param,
      Query__PetFavoritePhrase__output_type,
      Omit<ExtractParameters<Query__PetFavoritePhrase__param>, keyof {readonly id: string}>
    >,
  },
  readonly parameters: Query__OnlyOneRootLoadablePet__parameters,
};
