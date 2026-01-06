import { iso } from '@iso';
import { Card, CardContent, Stack } from '@mui/material';
import React from 'react';
import { useNavigateTo } from '../routes';

export const PetBestFriendCard = iso(`
  field Pet.PetBestFriendCard @component {
    id
    PetUpdater
    best_friend_relationship {
      picture_together
      best_friend {
        id
        fullName
        Avatar
      }
    }
  }
`)(function PetBestFriendCardComponent({ data }) {
  const navigateTo = useNavigateTo();
  const bestFriendRelationship = data.best_friend_relationship;
  if (bestFriendRelationship == null) {
    return (
      <Card
        variant="outlined"
        sx={{ width: 450, boxShadow: 3, backgroundColor: '#BBB' }}
      >
        <CardContent>
          <data.PetUpdater />
        </CardContent>
      </Card>
    );
  }

  return (
    <Card
      variant="outlined"
      sx={{ width: 450, boxShadow: 3, backgroundColor: '#BBB' }}
    >
      <CardContent>
        <Stack direction="column" spacing={4}>
          <Stack direction="row" spacing={4}>
            <bestFriendRelationship.best_friend.Avatar
              onClick={() =>
                navigateTo({
                  kind: 'PetDetail',
                  id: bestFriendRelationship.best_friend.id,
                })
              }
            />
            <div style={{ width: 300 }}>
              <h2>
                Best friend: {bestFriendRelationship.best_friend.fullName}
              </h2>
            </div>
          </Stack>
          <data.PetUpdater />
          <img
            src={
              (bestFriendRelationship.picture_together as string) ?? undefined
            }
            style={{ maxWidth: 400 }}
          />
        </Stack>
      </CardContent>
    </Card>
  );
});
