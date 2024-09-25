import React from 'react';
import { iso } from '@iso';
import { Avatar, Card, CardContent, Stack } from '@mui/material';
import { useNavigateTo } from './routes';

export const PetBestFriendCard = iso(`
  field Pet.PetBestFriendCard @component {
    id
    PetUpdater
    best_friend_relationship {
      picture_together
      best_friend {
        id
        name
        picture
      }
    }
  }
`)(function PetBestFriendCardComponent({ data }) {
  const navigateTo = useNavigateTo();
  const bestFriendRelationship = data.best_friend_relationship;
  if (!bestFriendRelationship) {
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
            <Avatar
              sx={{ height: 100, width: 100 }}
              src={bestFriendRelationship.best_friend.picture}
              onClick={() =>
                navigateTo({
                  kind: 'PetDetail',
                  id: bestFriendRelationship.best_friend.id,
                })
              }
              style={{ cursor: 'pointer' }}
            />
            <div style={{ width: 300 }}>
              <h2>Best friend: {bestFriendRelationship.best_friend.name}</h2>
            </div>
          </Stack>
          <data.PetUpdater />
          <img
            src={bestFriendRelationship.picture_together ?? undefined}
            style={{ maxWidth: 400 }}
          />
        </Stack>
      </CardContent>
    </Card>
  );
});
