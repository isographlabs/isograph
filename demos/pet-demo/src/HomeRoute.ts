import { iso } from './__isograph/iso';
import type { ResolverParameters } from './iso';

export const HomeRoute = iso(`
  field Query.HomeRoute @component {
    pets {
      id
      PetSummaryCard
    }
  }
`)(function HomeRoute({
  data,
}: ResolverParameters<{ pets: { PetSummaryCard: string }[] }>) {
  return data.pets.map((pet) => pet.PetSummaryCard);
});

console.log(iso(`entrypoint Query.HomeRoute`));
