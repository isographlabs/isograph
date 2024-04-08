import type  { Pet__PetBestFriendCard__outputType}  from '../../Pet/PetBestFriendCard/reader';
import type  { Pet__PetCheckinsCard__outputType}  from '../../Pet/PetCheckinsCard/reader';
import type  { Pet__PetPhraseCard__outputType}  from '../../Pet/PetPhraseCard/reader';
import type  { Pet__PetTaglineCard__outputType}  from '../../Pet/PetTaglineCard/reader';

export type Query__PetDetailRoute__param = {
  pet: ({
    name: string,
    PetCheckinsCard: Pet__PetCheckinsCard__outputType,
    PetBestFriendCard: Pet__PetBestFriendCard__outputType,
    PetPhraseCard: Pet__PetPhraseCard__outputType,
    PetTaglineCard: Pet__PetTaglineCard__outputType,
  } | null),
};

            