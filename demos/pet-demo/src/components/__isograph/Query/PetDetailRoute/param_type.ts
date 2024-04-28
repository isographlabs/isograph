import {Pet__PetBestFriendCard__outputType} from '../../Pet/PetBestFriendCard/output_type';
import {Pet__PetCheckinsCard__outputType} from '../../Pet/PetCheckinsCard/output_type';
import {Pet__PetPhraseCard__outputType} from '../../Pet/PetPhraseCard/output_type';
import {Pet__PetStatsCard__outputType} from '../../Pet/PetStatsCard/output_type';
import {Pet__PetTaglineCard__outputType} from '../../Pet/PetTaglineCard/output_type';

export type Query__PetDetailRoute__param = {
  pet: ({
    name: string,
    PetCheckinsCard: Pet__PetCheckinsCard__outputType,
    PetBestFriendCard: Pet__PetBestFriendCard__outputType,
    PetPhraseCard: Pet__PetPhraseCard__outputType,
    PetTaglineCard: Pet__PetTaglineCard__outputType,
    PetStatsCard: Pet__PetStatsCard__outputType,
  } | null),
};
