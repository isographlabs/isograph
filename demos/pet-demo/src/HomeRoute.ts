import { iso } from './__isograph/iso';

export const HomeRoute = iso(`
  field Query.HomeRoute {
    pets {
      id
      PetSummaryCard
    }
  }
`)(function HomeRoute({ data }) {
  return data.pets.map((pet) => ({ id: pet.id, summary: pet.PetSummaryCard }));
});

export const HomeRouteEntrypoint = iso(`entrypoint Query.HomeRoute`);
