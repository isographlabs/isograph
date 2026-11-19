import { iso } from './__isograph/iso';

export const PetDetailRoute = iso(`
  field Query.PetDetailRoute($id: ID!) {
    pet(id: $id) {
      id
      fullName
      PetStatsCard
      PetCheckinsCard(limit: 5)
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

export const PetDetailRouteEntrypoint = iso(`entrypoint Query.PetDetailRoute`);
