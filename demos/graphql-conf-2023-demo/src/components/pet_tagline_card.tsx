import React from "react";
import { iso } from "@isograph/react";
import { Card, CardContent } from "@mui/material";

import { ResolverParameterType as PetTaglineCardParams } from "@iso/Pet/pet_tagline_card/reader.isograph";

export const pet_tagline_card = iso<
  PetTaglineCardParams,
  ReturnType<typeof PetTaglineCard>
>`
Pet.pet_tagline_card @component {
  id,
  tagline,
}
`(PetTaglineCard);

function PetTaglineCard(props: PetTaglineCardParams) {
  return (
    <Card variant="outlined" sx={{ width: 450, boxShadow: 3 }}>
      <CardContent>
        <h2>Tagline</h2>
        <p>"{props.data.tagline}"</p>
      </CardContent>
    </Card>
  );
}
