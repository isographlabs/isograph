import { type Pet__PetBestFriendCard__output_type } from '../../Pet/PetBestFriendCard/output_type';
import { type Pet__PetCheckinsCard__output_type } from '../../Pet/PetCheckinsCard/output_type';
import { type Pet__PetPhraseCard__output_type } from '../../Pet/PetPhraseCard/output_type';
import { type Pet__PetStatsCard__output_type } from '../../Pet/PetStatsCard/output_type';
import { type Pet__PetTaglineCard__output_type } from '../../Pet/PetTaglineCard/output_type';
import { type Pet____refetch__output_type } from '../../Pet/__refetch/output_type';
import type { Query__PetDetailRoute__parameters } from './parameters_type';

export type Query__PetDetailRoute__param = {
  readonly data: {
    readonly node: ({
      /**
A client pointer for the Pet type.
      */
      readonly asPet: ({
        /**
A refetch field for the Pet type.
        */
        readonly __refetch: Pet____refetch__output_type,
        readonly tagline: string,
      } | null),
    } | null),
    readonly pet: ({
      readonly name: string,
      readonly PetCheckinsCard: Pet__PetCheckinsCard__output_type,
      readonly PetBestFriendCard: Pet__PetBestFriendCard__output_type,
      readonly PetPhraseCard: Pet__PetPhraseCard__output_type,
      readonly PetTaglineCard: Pet__PetTaglineCard__output_type,
      readonly PetStatsCard: Pet__PetStatsCard__output_type,
    } | null),
  },
  readonly parameters: Query__PetDetailRoute__parameters,
};
