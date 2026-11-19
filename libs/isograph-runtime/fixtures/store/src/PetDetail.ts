import { iso } from './__isograph/iso';

export const PetDetail = iso(`
  field Query.PetDetail($id: ID!, $skip: Int) {
    pet(id: $id) {
      id
      name
      nickname
      tagline
      stats {
        weight
        energy
      }
      bestFriend {
        id
        name
        bestFriend {
          id
          name
        }
      }
      checkins(limit: 2) {
        id
        location
      }
      pagedCheckins: checkins(limit: 2, skip: $skip) {
        id
        location
      }
    }
  }
`)(function PetDetail({ data }) {
  return data;
});

iso(`entrypoint Query.PetDetail`);
