import { iso } from './iso';
import type { ResolverParameters } from './iso';

export const PetDetailRoute = iso(`
  field Query.PetDetailRoute($id: ID!) @component {
    pet(id: $id) {
      id
      fullName
      PetStatsCard
      PetCheckinsCard(limit: 5) @loadable(lazyLoadArtifact: true)
      PetBestFriendCard
      bestFriend {
        fullName
      }
      owner {
        OwnerBadge
      }
    }
  }
`)(function PetDetailRoute({
  data,
}: ResolverParameters<{ pet: { fullName: string } | null }>) {
  return data.pet?.fullName ?? 'No pet with that id';
});

iso(`entrypoint Query.PetDetailRoute`);
