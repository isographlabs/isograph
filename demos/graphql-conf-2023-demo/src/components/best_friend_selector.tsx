import React, { useState } from "react";
import { iso } from "@isograph/react";
import { MenuItem, Select } from "@mui/material";

import { ResolverParameterType as BestFriendSelectorParams } from "@iso/Pet/best_friend_selector/reader.isograph";
import { PetId } from "./router";

export const best_friend_selector = iso<
  BestFriendSelectorParams,
  ReturnType<typeof BestFriendSelector>
>`
  Pet.best_friend_selector @component {
    __set_pet_best_friend,
    potential_new_best_friends {
      id,
      name,
    },
  }
`(BestFriendSelector);

function BestFriendSelector(props: BestFriendSelectorParams) {
  const [selected, setSelected] = useState<PetId | "NONE">("NONE");
  return (
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
  );
}
