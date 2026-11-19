import { iso } from './__isograph/iso';

export const ConstantPet = iso(`
  field Query.ConstantPet($id: ID!) {
    pet(id: $id) {
      Constant
    }
  }
`)(function ConstantPet({ data }) {
  return data;
});

iso(`entrypoint Query.ConstantPet`);
