import React from "react";
import { iso } from "@isograph/react";
import { Card, CardContent } from "@mui/material";

import { ResolverParameterType as PetPhraseCardParams } from "@iso/Pet/pet_phrase_card/reader.isograph";

export const pet_phrase_card = iso<
  PetPhraseCardParams,
  ReturnType<typeof PetPhraseCard>
>`
Pet.pet_phrase_card @component {
  id,
  favorite_phrase,
}
`(PetPhraseCard);

function PetPhraseCard(props: PetPhraseCardParams) {
  return (
    <Card variant="outlined" sx={{ width: 450, boxShadow: 3 }}>
      <CardContent>
        <h2>Likes to say</h2>
        <p>"{props.data.favorite_phrase}"</p>
      </CardContent>
    </Card>
  );
}
