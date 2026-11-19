import { iso } from './__isograph/iso';

export const PetSummaryCard = iso(`
  field Pet.PetSummaryCard @component
  """
  A card with the pet's name, picture, and tagline.
  """
  {
    id
    fullName
    nickname
    age
    picture
    tagline
    species
  }
`)(function PetSummaryCard({ data }) {
  return `${data.fullName}: ${data.tagline}`;
});
