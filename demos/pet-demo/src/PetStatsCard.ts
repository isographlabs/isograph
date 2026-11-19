import { iso } from './iso';
import type { ResolverParameters } from './iso';

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
`)(function PetStatsCard({
  data,
}: ResolverParameters<{ stats: { intelligence: number | null } | null }>) {
  return data.stats?.intelligence ?? 0;
});
