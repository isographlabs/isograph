import { iso } from './__isograph/iso';

export const PetLink = iso(`
  field Pet.PetLink
`)(function PetLink({ link }) {
  return link;
});
