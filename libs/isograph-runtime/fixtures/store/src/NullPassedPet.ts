import { iso } from './__isograph/iso';

export const NullPassedPet = iso(`
  field Query.NullPassedPet($id: ID!) {
    pet(id: $id) {
      NullableCheckins(n: null)
    }
  }
`)(function NullPassedPet({ data }) {
  return data;
});

iso(`entrypoint Query.NullPassedPet`);
