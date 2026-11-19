import { iso } from './__isograph/iso';

export const HomeRoute = iso(`
  field Query.HomeRoute @component {
    pets {
      id
      PetSummaryCard
    }
  }
`)(function HomeRoute({ data }) {
  return data.pets.map((pet) => pet.PetSummaryCard);
});

console.log(iso(`entrypoint Query.HomeRoute`));
