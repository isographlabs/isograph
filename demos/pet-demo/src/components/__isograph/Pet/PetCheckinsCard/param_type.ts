import { type Checkin__CheckinDisplay__output_type } from '../../Checkin/CheckinDisplay/output_type';

export type Pet__PetCheckinsCard__param = {
  id: string,
  checkins: ({
    CheckinDisplay: Checkin__CheckinDisplay__output_type,
    id: string,
  })[],
};
