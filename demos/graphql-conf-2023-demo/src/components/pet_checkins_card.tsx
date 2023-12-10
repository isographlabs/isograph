import React from "react";
import { iso } from "@isograph/react";
import { Card, CardContent } from "@mui/material";

import { ResolverParameterType as PetCheckinsCardParams } from "@iso/Pet/pet_checkins_card/reader.isograph";

export const pet_checkins_card = iso<
  PetCheckinsCardParams,
  ReturnType<typeof PetCheckinsCard>
>`
  Pet.pet_checkins_card @component {
    id,
    checkins {
      id,
      location,
      time,
    },
  }
`(PetCheckinsCard);

function PetCheckinsCard(props: PetCheckinsCardParams) {
  return (
    <Card variant="outlined" sx={{ width: 450, boxShadow: 3 }}>
      <CardContent>
        <h2>Check-ins</h2>
        <ul>
          {props.data.checkins.map((checkin) => (
            <li key={checkin.id}>
              <b>{checkin.location}</b> at {checkin.time}
            </li>
          ))}
        </ul>
      </CardContent>
    </Card>
  );
}
