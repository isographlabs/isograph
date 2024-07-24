import React from 'react';
import { iso } from '@iso';
import { Button, Card, CardContent } from '@mui/material';

export const PetCheckinsCard = iso(`
  field Pet.PetCheckinsCard($count: Int! = 42) @component {
    id
    checkins {
      CheckinDisplay
      id
    }
  }
`)(function PetCheckinsCardComponent(data) {
  return (
    <Card
      variant="outlined"
      sx={{ width: 450, boxShadow: 3, backgroundColor: '#BBB' }}
    >
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
    <Button onClick={() => checkin.make_super[1]({})} variant="text">
      🎉
    </Button>
  </b>
));
