import { iso } from './__isograph/iso';

export const PetName = iso(`
  field Pet.PetName {
    name
    nickname
  }
`)(function PetName({ data }) {
  return data;
});
