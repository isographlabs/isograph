import { type Query__PetFavoritePhrase2__output_type } from '../../Query/PetFavoritePhrase2/output_type';
import { type LoadableField, type ExtractParameters } from '@isograph/react';
import { type Query__PetFavoritePhrase2__param } from '../../Query/PetFavoritePhrase2/param_type';
import type { Query__OnlyOneRootLoadablePet__parameters } from './parameters_type';

export type Query__OnlyOneRootLoadablePet__param = {
  readonly data: {
    /**
PetFavoritePhrase2 because we currently have a bug where we don't catch the fact
that an entrypoint generated via @loadable and a regular entrypoint need to have
identical @lazyLoad settings. Oops!
    */
    readonly PetFavoritePhrase2: LoadableField<
      Query__PetFavoritePhrase2__param,
      Query__PetFavoritePhrase2__output_type,
      Omit<ExtractParameters<Query__PetFavoritePhrase2__param>, keyof {readonly id: string}>
    >,
  },
  readonly parameters: Query__OnlyOneRootLoadablePet__parameters,
};
