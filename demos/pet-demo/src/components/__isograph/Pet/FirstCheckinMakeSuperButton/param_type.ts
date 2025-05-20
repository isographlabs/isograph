import { type Checkin__make_super__output_type } from '../../Checkin/make_super/output_type';

export type Pet__FirstCheckinMakeSuperButton__param = {
  readonly data: {
    readonly checkins: ReadonlyArray<{
      readonly make_super: Checkin__make_super__output_type,
      readonly location: string,
    }>,
  },
  readonly parameters: Record<PropertyKey, never>,
};
