import React from "react";
import { iso } from "@isograph/react";
import { Card, CardContent } from "@mui/material";

import { ResolverParameterType as PetCheckinsCardParams } from "./__isograph/Pet/pet_checkins_card.isograph";

export const pet_phrase_card = iso<
  PetCheckinsCardParams,
  ReturnType<typeof PetCheckinsCard>
>`
Pet.pet_phrase_card @component {
  id,
  favorite_phrase,
}
`(PetCheckinsCard);

function PetCheckinsCard(props: PetCheckinsCardParams) {
  console.log('check ins', { props })
  return (
    <Card
      variant="outlined"
      sx={{ width: 450, boxShadow: 3 }}
    >
      <CardContent>
        <h2>Likes to say</h2>
        <p>
          "{props.data.favorite_phrase}"
        </p>
      </CardContent>
    </Card>
  );
}
