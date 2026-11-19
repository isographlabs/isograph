import { iso } from './iso';
import type { ResolverParameters } from './iso';

export const SetTagline = iso(`
  field Mutation.SetTagline($id: ID!, $tagline: String!) {
    set_pet_tagline(input: { id: $id, tagline: $tagline }) {
      pet {
        id
        tagline
      }
    }
  }
`)(function SetTagline({
  data,
}: ResolverParameters<{ set_pet_tagline: { pet: { tagline: string } } }>) {
  return data.set_pet_tagline.pet.tagline;
});

iso(`entrypoint Mutation.SetTagline`);
