import React, { useState } from "react";
import { iso } from "@isograph/react";
import { MenuItem, Select, Button, Input, Box } from "@mui/material";

import { ResolverParameterType as PetUpdaterParams } from "@iso/Pet/pet_updater/reader.isograph";
import { PetId } from "./router";

export const pet_updater = iso<PetUpdaterParams, ReturnType<typeof PetUpdater>>`
  Pet.pet_updater @component {
    __set_pet_best_friend,
    potential_new_best_friends {
      id,
      name,
    },

    __set_pet_tagline,
    tagline,
  }
`(PetUpdater);

function PetUpdater(props: PetUpdaterParams) {
  const [selected, setSelected] = useState<PetId | "NONE">("NONE");
  const [tagline, setTagline] = useState<string>(props.data.tagline);

  const updateTagline = () =>
    props.data.__set_pet_tagline({ input: { tagline } });

  return (
    <>
      <Select
        value={selected}
        onChange={(e) => {
          const value = e.target.value;
          if (typeof value === "string") {
            setSelected("NONE");
            if (value === "NONE") {
              return;
            }
            props.data.__set_pet_best_friend({
              new_best_friend_id: value,
            });
          }
        }}
      >
        <MenuItem value="NONE">Select new best friend</MenuItem>
        {props.data.potential_new_best_friends.map((potentialNewBestFriend) => (
          <MenuItem
            value={potentialNewBestFriend.id}
            key={potentialNewBestFriend.id}
          >
            {potentialNewBestFriend.name}
          </MenuItem>
        ))}
      </Select>
      <Box sx={{ display: "flex", justifyContent: "space-between" }}>
        <Input
          value={tagline}
          onChange={(e) => setTagline(e.target.value)}
          color="primary"
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              updateTagline();
            }
          }}
        />
        <Button variant="contained" onClick={updateTagline}>
          Set tagline
        </Button>
      </Box>
    </>
  );
}
