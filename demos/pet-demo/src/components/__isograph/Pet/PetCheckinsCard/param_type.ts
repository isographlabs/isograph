import { type Checkin__CheckinDisplay__output_type } from '../../Checkin/CheckinDisplay/output_type';

import { type Variables } from '@isograph/react';

export type Pet__PetCheckinsCard__param = {
  readonly data: {
    readonly id: string,
    readonly checkins: ReadonlyArray<{
      readonly CheckinDisplay: Checkin__CheckinDisplay__output_type,
      readonly id: string,
    }>,
  },
  readonly parameters: Variables,
};
