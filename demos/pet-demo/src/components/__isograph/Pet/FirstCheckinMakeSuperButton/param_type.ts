import { type ICheckin__make_super__output_type } from '../../ICheckin/make_super/output_type';

import { type Variables } from '@isograph/react';

export type Pet__FirstCheckinMakeSuperButton__param = {
  readonly data: {
    readonly checkins: ReadonlyArray<{
      readonly make_super: ICheckin__make_super__output_type,
      readonly location: string,
    }>,
  },
  readonly parameters: Variables,
};
