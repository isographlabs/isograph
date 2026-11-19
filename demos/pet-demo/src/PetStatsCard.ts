import { iso } from './__isograph/iso';

export const PetStatsCard = iso(`
  field Pet.PetStatsCard @component {
    id
    nickname
    stats {
      weight
      intelligence
      cuteness
      hunger
      sociability
      energy
    }
  }
`)(function PetStatsCard({ data }) {
  return data.stats?.intelligence ?? 0;
});
