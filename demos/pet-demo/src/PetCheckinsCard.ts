import { iso } from './__isograph/iso';

export const PetCheckinsCard = iso(`
  field Pet.PetCheckinsCard($limit: Int = 3) @component {
    id
    checkins(limit: $limit, skip: 0) {
      id
      location
      time
    }
  }
`)(function PetCheckinsCard({ data }) {
  return data.checkins.map((checkin) => checkin.location);
});
