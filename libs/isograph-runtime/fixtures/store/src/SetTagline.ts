import { iso } from './__isograph/iso';

export const SetTagline = iso(`
  field Mutation.SetTagline($id: ID!, $tagline: String!) {
    setTagline(input: { id: $id, tagline: $tagline }) {
      pet {
        id
        tagline
      }
      query {
        greeting
      }
    }
  }
`)(function SetTagline({ data }) {
  return data;
});

iso(`entrypoint Mutation.SetTagline`);
