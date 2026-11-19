import { iso } from './__isograph/iso';

export const StatsCard = iso(`
  field PetStats.StatsCard {
    weight
  }
`)(function StatsCard({ data }) {
  return data;
});
