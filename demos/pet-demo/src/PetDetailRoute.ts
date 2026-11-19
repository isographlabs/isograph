import { iso } from './__isograph/iso';

export const PetDetailRoute = iso(`
  field Query.PetDetailRoute($id: ID!) {
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
`)(function PetDetailRoute({ data }) {
  return data.pet?.fullName ?? 'No pet with that id';
});

console.log(iso(`entrypoint Query.PetDetailRoute`));
