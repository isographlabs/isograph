import { iso } from './__isograph/iso';

export const NullableCheckins = iso(`
  field Pet.NullableCheckins($n: Int) {
    checkins(limit: $n) {
      id
    }
  }
`)(function NullableCheckins({ data }) {
  return data;
});
