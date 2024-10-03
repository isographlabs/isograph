import React from 'react';
import { iso } from '@iso';
import { Button, Card, CardContent } from '@mui/material';
import { useImperativeExposedMutationField } from '@isograph/react';

export const PetCheckinsCard = iso(`
  field Pet.PetCheckinsCard($skip: Int, $limit: Int) @component {
    id
    checkins(skip: $skip, limit: $limit) {
      CheckinDisplay
      id
    }
  }
`)(function PetCheckinsCardComponent({ data }) {
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
`)(({ data: checkin }) => {
  const { loadField } = useImperativeExposedMutationField(checkin.make_super);
  return (
    <b>
      {checkin.location} at {checkin.time}&nbsp;
      <Button onClick={() => loadField({})} variant="text">
        🎉
      </Button>
    </b>
  );
});

export const PetCheckinsCardList = iso(`
  field Pet.PetCheckinsCardList($skip: Int, $limit: Int) {
    checkins(skip: $skip, limit: $limit) {
      CheckinDisplay
      id
    }
  }
`)(function PetCheckinsCardComponent({ data }) {
  return data.checkins;
});
