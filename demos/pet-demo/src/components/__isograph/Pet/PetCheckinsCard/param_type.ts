import {Checkin__CheckinDisplay__outputType} from '../../Checkin/CheckinDisplay/output_type';

export type Pet__PetCheckinsCard__param = {
  id: string,
  checkins: ({
    CheckinDisplay: Checkin__CheckinDisplay__outputType,
    id: string,
  })[],
};
