import React from 'react';
import { iso } from '@iso';
import { Card, CardContent } from '@mui/material';

export const PetCheckinsCard = iso(`
  field Pet.PetCheckinsCard @component {
    id
    checkins {
      id
      location
      time
    }
  }
`)(function PetCheckinsCardComponent(props) {
  return (
    <Card variant="outlined" sx={{ width: 450, boxShadow: 3 }}>
      <CardContent>
        <h2>Check-ins</h2>
        <ul>
          {props.data.checkins.map((checkin) => (
            <li key={checkin.id}>
              <b>{checkin.location}</b> at {checkin.time}
            </li>
          ))}
        </ul>
      </CardContent>
    </Card>
  );
});
