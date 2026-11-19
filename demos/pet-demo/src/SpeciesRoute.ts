import { iso } from './__isograph/iso';

export const SpeciesRoute = iso(`
  field Query.SpeciesRoute($species: PetSpecies!) {
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
`)(function SpeciesRoute({ data }) {
  return data.petsBySpecies.map((pet) => pet.fullName);
});

console.log(iso(`entrypoint Query.SpeciesRoute`));
