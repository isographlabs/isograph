import React from 'react';
import { iso } from '@iso';
import { Button, Card, CardContent } from '@mui/material';

export const PetCheckinsCard = iso(`
  field Pet.PetCheckinsCard @component {
    id
    checkins {
      CheckinDisplay
      id
      location
      time
    }
  }
`)(function PetCheckinsCardComponent(data) {
  return (
    <Card variant="outlined" sx={{ width: 450, boxShadow: 3 }}>
      <CardContent>
        <h2>Check-ins</h2>
        <ul>
          {data.checkins.map((checkin) => (
            <li key={checkin.id}>
              <checkin.CheckinDisplay />
            </li>
          ))}
        </ul>
      </CardContent>
    </Card>
  );
});

export const CheckinDisplay = iso(`
  field Checkin.CheckinDisplay @component {
    location,
    time
    make_super
  }
`)((checkin) => (
  <b>
    {checkin.location} at {checkin.time}&nbsp;
    <Button onClick={() => checkin.make_super({})} variant="text">
      🎉
    </Button>
  </b>
));
