import React from "react";
import { iso } from "@isograph/react";
import { Avatar, Card, CardContent, Stack } from "@mui/material";

import { ResolverParameterType as PetSummaryCardParams } from "@iso/Pet/pet_summary_card/reader.isograph";

export const pet_summary_card = iso<
  PetSummaryCardParams,
  ReturnType<typeof PetSummaryCard>
>`
  Pet.pet_summary_card @component {
    id,
    name,
    picture,
    tagline,
  }
`(PetSummaryCard);

function PetSummaryCard(props: PetSummaryCardParams) {
  return (
    <Card
      variant="outlined"
      sx={{ width: 450, boxShadow: 3, cursor: "pointer" }}
      onClick={() => props.navigateTo({ kind: "PetDetail", id: props.data.id })}
    >
      <CardContent>
        <Stack direction="row" spacing={4}>
          <Avatar sx={{ height: 100, width: 100 }} src={props.data.picture} />
          <div style={{ width: 300 }}>
            <h2>{props.data.name}</h2>
            <div>
              <i>{props.data.tagline}</i>
            </div>
          </div>
        </Stack>
      </CardContent>
    </Card>
  );
}
