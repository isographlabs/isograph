import React from "react";
import { iso } from "@isograph/react";
import { Avatar, Card, CardContent, Stack } from "@mui/material";

import { ResolverParameterType as PetBestFriendCardParams } from "@iso/Pet/pet_best_friend_card/reader.isograph";

export const pet_best_friend_card = iso<
  PetBestFriendCardParams,
  ReturnType<typeof PetBestFriendCard>
>`
  Pet.pet_best_friend_card @component {
    id,
    best_friend_selector,
    best_friend_relationship {
      picture_together,
      best_friend {
        id,
        name,
        picture,
      },
    },
  }
`(PetBestFriendCard);

function PetBestFriendCard(props: PetBestFriendCardParams) {
  const bestFriendRelationship = props.data.best_friend_relationship;
  if (!bestFriendRelationship) {
    return (
      <Card variant="outlined" sx={{ width: 450, boxShadow: 3 }}>
        <CardContent>
          {props.data.best_friend_selector({ parentId: props.data.id })}
        </CardContent>
      </Card>
    );
  }

  return (
    <Card variant="outlined" sx={{ width: 450, boxShadow: 3 }}>
      <CardContent>
        <Stack direction="column" spacing={4}>
          <Stack direction="row" spacing={4}>
            <Avatar
              sx={{ height: 100, width: 100 }}
              src={bestFriendRelationship.best_friend.picture}
            />
            <div style={{ width: 300 }}>
              <h2>Best friend: {bestFriendRelationship.best_friend.name}</h2>
            </div>
          </Stack>
          {props.data.best_friend_selector({ parentId: props.data.id })}
          <img
            src={bestFriendRelationship.picture_together ?? undefined}
            style={{ maxWidth: 400 }}
          />
        </Stack>
      </CardContent>
    </Card>
  );
}
