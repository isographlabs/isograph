import { iso } from './__isograph/iso';

export const SetTagline = iso(`
  field Mutation.SetTagline($id: ID!, $tagline: String!) {
    set_pet_tagline(input: { id: $id, tagline: $tagline }) {
      pet {
        id
        tagline
      }
    }
  }
`)(function SetTagline({ data }) {
  return data.set_pet_tagline.pet.tagline;
});

console.log(iso(`entrypoint Mutation.SetTagline`));
