import React, { useState } from 'react';
import { iso } from '@iso';
import { MenuItem, Select, Button, Input, Box } from '@mui/material';
import { PetId } from './routes';

export const PetUpdater = iso(`
  field Pet.PetUpdater @component
  """
  # Pet.PetUpdater
  A component to test behavior with respect to mutations.
  You can update the best friend and the tagline.
  """
  {
    set_best_friend
    potential_new_best_friends {
      id
      name
    },

    set_pet_tagline
    tagline

    __refetch
  }
`)(function PetUpdaterComponent({ data: pet }) {
  const [selected, setSelected] = useState<PetId | 'NONE'>('NONE');
  const [tagline, setTagline] = useState<string>(pet.tagline);
  // TODO the tagline can change. But we're storing a stale one in state.
  // We should find a way to work around this.

  const updateTagline = () => pet.set_pet_tagline({ input: { tagline } })[1]();

  return (
    <>
      <Select
        value={selected}
        onChange={(e) => {
          const value = e.target.value;
          if (typeof value === 'string') {
            setSelected('NONE');
            if (value === 'NONE') {
              return;
            }
            pet.set_best_friend({
              new_best_friend_id: value,
            })[1]();
          }
        }}
      >
        <MenuItem value="NONE">Select new best friend</MenuItem>
        {pet.potential_new_best_friends.map((potentialNewBestFriend) => (
          <MenuItem
            value={potentialNewBestFriend.id}
            key={potentialNewBestFriend.id}
          >
            {potentialNewBestFriend.name}
          </MenuItem>
        ))}
      </Select>
      <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
        <Input
          value={tagline}
          onChange={(e) => setTagline(e.target.value)}
          color="primary"
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              updateTagline();
            }
          }}
        />
        <Button variant="contained" onClick={updateTagline}>
          Set tagline
        </Button>
        <Button variant="contained" onClick={() => pet.__refetch()[1]()}>
          Refetch pet
        </Button>
      </Box>
    </>
  );
});
