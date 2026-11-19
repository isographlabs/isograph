import { iso } from './__isograph/iso';

export const PetSummaries = iso(`
  field Query.PetSummaries {
    pets {
      PetName
    }
  }
`)(function PetSummaries({ data }) {
  return data;
});

iso(`entrypoint Query.PetSummaries`);
