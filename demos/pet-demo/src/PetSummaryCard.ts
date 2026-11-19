import { iso } from './iso';
import type { ResolverParameters } from './iso';

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
`)(function PetSummaryCard({
  data,
}: ResolverParameters<{ fullName: string; tagline: string }>) {
  return `${data.fullName}: ${data.tagline}`;
});
