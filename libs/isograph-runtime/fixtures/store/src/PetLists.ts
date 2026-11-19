import { iso } from './__isograph/iso';

export const PetLists = iso(`
  field Query.PetLists {
    pets {
      id
      name
    }
    maybePets {
      id
      name
    }
    petGrid {
      id
      name
    }
    statGrid {
      weight
    }
  }
`)(function PetLists({ data }) {
  return data;
});

iso(`entrypoint Query.PetLists`);
