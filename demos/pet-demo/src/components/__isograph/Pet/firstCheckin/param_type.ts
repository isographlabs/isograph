import { type Checkin__link__output_type } from '../../Checkin/link/output_type';

export type Pet__firstCheckin__param = {
  readonly data: {
    readonly checkins: ReadonlyArray<{
      /**
A store Link for the Checkin type.
      */
      readonly link: Checkin__link__output_type,
    }>,
  },
  readonly parameters: Record<PropertyKey, never>,
};
