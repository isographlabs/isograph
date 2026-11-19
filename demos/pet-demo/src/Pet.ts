import { iso } from './iso';
import type { ResolverParameters } from './iso';

export const fullName = iso(`
  field Pet.fullName "The pet's first and last name, for example Makayla Balicki." {
    firstName
    lastName
  }
`)(function fullName({
  data,
}: ResolverParameters<{ firstName: string; lastName: string }>) {
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
`)(function bestFriend({
  data,
}: ResolverParameters<{
  best_friend_relationship: { best_friend: { id: string } } | null;
}>) {
  return data.best_friend_relationship?.best_friend.id ?? null;
});
