import React from 'react';
import { iso } from '@iso';
import { Button } from '@mui/material';

export const FirstCheckinMakeSuperButton = iso(`
  field Pet.FirstCheckinMakeSuperButton @component {
    checkins(skip: 0, limit: 1) {
      make_super
      # location is unused in this component, but we need to select it
      # because we need it to show up in the refetch query response, so
      # that it can update the rendered CheckinDisplay
      location
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
