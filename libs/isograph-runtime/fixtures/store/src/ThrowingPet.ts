import { iso } from './__isograph/iso';

export const ThrowingPet = iso(`
  field Query.ThrowingPet($id: ID!) {
    pet(id: $id) {
      ThrowingName
    }
  }
`)(function ThrowingPet({ data }) {
  return data;
});

iso(`entrypoint Query.ThrowingPet`);
