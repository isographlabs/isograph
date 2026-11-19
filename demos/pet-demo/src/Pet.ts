import { iso } from './__isograph/iso';

export const fullName = iso(`
  field Pet.fullName "The pet's first and last name, for example Makayla Balicki." {
    firstName
    lastName
  }
`)(function fullName({ data }) {
  return `${data.firstName} ${data.lastName}`;
});

export const bestFriend = iso(`
  field Pet.bestFriend to Pet {
    best_friend_relationship {
      best_friend {
        id
      }
    }
  }
`)(function bestFriend({ data }) {
  const friend = data.best_friend_relationship?.best_friend;
  return friend == null ? null : { entityName: 'Pet' as const, id: friend.id };
});
