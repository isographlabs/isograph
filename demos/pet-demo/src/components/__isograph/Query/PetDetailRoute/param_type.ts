import {Pet__PetBestFriendCard__outputType} from '../../Pet/PetBestFriendCard/output_type';
import {Pet__PetCheckinsCard__outputType} from '../../Pet/PetCheckinsCard/output_type';
import {Pet__PetPhraseCard__outputType} from '../../Pet/PetPhraseCard/output_type';
import {Pet__PetTaglineCard__outputType} from '../../Pet/PetTaglineCard/output_type';
import {Pet____refetch__outputType} from '../../Pet/__refetch/output_type';

export type Query__PetDetailRoute__param = {
  pet: ({
    /**
A refetch field for this object.
    */
    __refetch: Pet____refetch__outputType,
    name: string,
    PetCheckinsCard: Pet__PetCheckinsCard__outputType,
    PetBestFriendCard: Pet__PetBestFriendCard__outputType,
    PetPhraseCard: Pet__PetPhraseCard__outputType,
    PetTaglineCard: Pet__PetTaglineCard__outputType,
  } | null),
};
