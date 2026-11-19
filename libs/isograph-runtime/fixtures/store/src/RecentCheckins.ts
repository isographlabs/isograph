import { iso } from './__isograph/iso';

export const RecentCheckins = iso(`
  field Pet.RecentCheckins($n: Int = 2) {
    checkins(limit: $n) {
      id
      location
    }
  }
`)(function RecentCheckins({ data }) {
  return data;
});
