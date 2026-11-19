import { iso } from './__isograph/iso';

export const Links = iso(`
  field Query.Links($id: ID!) {
    pet(id: $id) {
      PetLink
      stats {
        StatsLink
      }
      bestFriend {
        PetLink
      }
    }
    node(id: $id) {
      NodeLink
    }
  }
`)(function Links({ data }) {
  return data;
});

iso(`entrypoint Query.Links`);
