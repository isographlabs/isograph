import { iso } from './iso';
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

iso(`entrypoint Query.HomeRoute`);
