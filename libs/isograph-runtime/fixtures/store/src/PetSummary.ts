import { iso } from './__isograph/iso';

export const PetSummary = iso(`
  field Query.PetSummary($id: ID!) {
    pet(id: $id) {
      id
      PetName
      RecentCheckins
    }
  }
`)(function PetSummary({ data }) {
  return data;
});

iso(`entrypoint Query.PetSummary`);
