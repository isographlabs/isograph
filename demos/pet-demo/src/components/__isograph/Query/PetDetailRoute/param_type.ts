import { type Pet__PetBestFriendCard__output_type } from '../../Pet/PetBestFriendCard/output_type';
import { type Pet__PetCheckinsCard__output_type } from '../../Pet/PetCheckinsCard/output_type';
import { type Pet__PetPhraseCard__output_type } from '../../Pet/PetPhraseCard/output_type';
import { type Pet__PetStatsCard__output_type } from '../../Pet/PetStatsCard/output_type';
import { type Pet__PetTaglineCard__output_type } from '../../Pet/PetTaglineCard/output_type';

export type Query__PetDetailRoute__param = {
  pet: ({
    name: string,
    PetCheckinsCard: Pet__PetCheckinsCard__output_type,
    PetBestFriendCard: Pet__PetBestFriendCard__output_type,
    PetPhraseCard: Pet__PetPhraseCard__output_type,
    PetTaglineCard: Pet__PetTaglineCard__output_type,
    PetStatsCard: Pet__PetStatsCard__output_type,
  } | null),
};
