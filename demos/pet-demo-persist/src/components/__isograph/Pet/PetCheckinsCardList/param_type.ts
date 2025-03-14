import { type Checkin__CheckinDisplay__output_type } from '../../Checkin/CheckinDisplay/output_type';
import type { Pet__PetCheckinsCardList__parameters } from './parameters_type';

export type Pet__PetCheckinsCardList__param = {
  readonly data: {
    readonly checkins: ReadonlyArray<{
      readonly CheckinDisplay: Checkin__CheckinDisplay__output_type,
      readonly id: string,
    }>,
  },
  readonly parameters: Pet__PetCheckinsCardList__parameters,
};
