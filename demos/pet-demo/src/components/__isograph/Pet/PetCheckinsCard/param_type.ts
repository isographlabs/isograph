import { type Checkin__CheckinDisplay__output_type } from '../../Checkin/CheckinDisplay/output_type';

import { type Variables } from '@isograph/react';

export type Pet__PetCheckinsCard__param = {
  data: {
    id: string,
    checkins: ReadonlyArray<{
      CheckinDisplay: Checkin__CheckinDisplay__output_type,
      id: string,
    }>,
  },
  parameters: Variables,
};
