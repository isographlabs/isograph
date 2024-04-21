import {Pet__PetSummaryCard__outputType} from '../../Pet/PetSummaryCard/output_type';
import {Pet__petSuperName__outputType} from '../../Pet/petSuperName/output_type';

export type Query__HomeRoute__param = {
  pets: ({
    id: string,
    PetSummaryCard: Pet__PetSummaryCard__outputType,
  })[],
  pet: ({
    petSuperName: Pet__petSuperName__outputType,
  } | null),
};
