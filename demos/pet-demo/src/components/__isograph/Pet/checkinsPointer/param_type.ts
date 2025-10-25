import { type Checkin____link__output_type } from '../../Checkin/__link/output_type';

export type Pet__checkinsPointer__param = {
  readonly data: {
    readonly checkins: ReadonlyArray<{
      /**
A store Link for the Checkin type.
      */
      readonly __link: Checkin____link__output_type,
    }>,
  },
  readonly parameters: Record<PropertyKey, never>,
};
