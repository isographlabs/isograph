import { type Pet__PetBestFriendCard__output_type } from '../../Pet/PetBestFriendCard/output_type';
import { type Pet__PetCheckinsCard__output_type } from '../../Pet/PetCheckinsCard/output_type';
import { type Pet__PetPhraseCard__output_type } from '../../Pet/PetPhraseCard/output_type';
import { type Pet__PetStatsCard__output_type } from '../../Pet/PetStatsCard/output_type';
import { type Pet__PetTaglineCard__output_type } from '../../Pet/PetTaglineCard/output_type';
import { type Pet__custom_pet_refetch__output_type } from '../../Pet/custom_pet_refetch/output_type';
import type { Query__PetDetailRoute__parameters } from './parameters_type';

export type Query__PetDetailRoute__param = {
  readonly data: {
    readonly pet: ({
      readonly custom_pet_refetch: Pet__custom_pet_refetch__output_type,
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
