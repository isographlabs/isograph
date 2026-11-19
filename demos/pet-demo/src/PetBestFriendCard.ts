import { iso } from './iso';
import type { ResolverParameters } from './iso';

export const PetBestFriendCard = iso(`
  field Pet.PetBestFriendCard @component {
    id
    best_friend_relationship {
      picture_together
      best_friend {
        id
        friendName: firstName
        fullName
      }
    }
  }
`)(function PetBestFriendCard({
  data,
}: ResolverParameters<{
  best_friend_relationship: { best_friend: { friendName: string } } | null;
}>) {
  return data.best_friend_relationship?.best_friend.friendName ?? 'Nobody';
});
