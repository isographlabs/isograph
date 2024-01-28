import React from "react";
import { iso } from "@isograph/react";
import { Card, CardContent } from "@mui/material";

import { ResolverParameterType as PetPhraseCardParams } from "@iso/Pet/PetPhraseCard/reader.isograph";

export const PetPhraseCard = iso<
  PetPhraseCardParams,
  ReturnType<typeof PetPhraseCardComponent>
>`
field Pet.PetPhraseCard @component {
  id,
  favorite_phrase,
}
`(PetPhraseCardComponent);

function PetPhraseCardComponent(props: PetPhraseCardParams) {
  return (
    <Card variant="outlined" sx={{ width: 450, boxShadow: 3 }}>
      <CardContent>
        <h2>Likes to say</h2>
        <p>"{props.data.favorite_phrase}"</p>
      </CardContent>
    </Card>
  );
}
