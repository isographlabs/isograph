import { iso } from './__isograph/iso';

export const PetBestFriendCard = iso(`
  field Pet.PetBestFriendCard {
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
`)(function PetBestFriendCard({ data }) {
  return data.best_friend_relationship?.best_friend.friendName ?? 'Nobody';
});
