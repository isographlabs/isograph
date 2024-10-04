import React from 'react';
import { iso } from '@iso';
import { Button } from '@mui/material';

export const FirstCheckinMakeSuperButton = iso(`
  field Pet.FirstCheckinMakeSuperButton @component {
    checkins(skip: 0, limit: 1) {
      make_super
    }
  }
`)(({ data }) => {
  return (
    <Button
      onClick={() => {
        data.checkins[0].make_super({})[1]();
      }}
      variant="contained"
    >
      Make first checkin super
    </Button>
  );
});
