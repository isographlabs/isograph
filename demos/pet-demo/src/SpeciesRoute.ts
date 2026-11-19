import { iso } from './iso';
import type { ResolverParameters } from './iso';

export const SpeciesRoute = iso(`
  field Query.SpeciesRoute($species: PetSpecies!) @component {
    petsBySpecies(species: $species) {
      id
      fullName
      favorite_phrase
    }
    viewer {
      id
    }
    node(id: "1") {
      id
    }
  }
`)(function SpeciesRoute({
  data,
}: ResolverParameters<{ petsBySpecies: { fullName: string }[] }>) {
  return data.petsBySpecies.map((pet) => pet.fullName);
});

iso(`entrypoint Query.SpeciesRoute`);
