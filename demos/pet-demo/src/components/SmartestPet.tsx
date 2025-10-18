import { LoadableFieldReader } from '@isograph/react';
import { Card, CardContent, Container, Stack } from '@mui/material';
import { iso } from '@iso';
import { useNavigateTo } from './routes';
import { Suspense } from 'react';

export const SmartestPetRoute = iso(`
  field Query.SmartestPetRoute @component {
    smartestPet {
      id
      fullName
      Avatar
      stats {
        intelligence
      }
      picture
      checkinsPointer {
        location
      }
    }
  }
`)(function SmartestRouteComponent({ data }) {
  const navigateTo = useNavigateTo();

  return (
    <Container maxWidth="md">
      <h1>Smartest Pet Award</h1>
      <Card
        variant="outlined"
        sx={{
          width: 450,
          boxShadow: 3,
          cursor: 'pointer',
          backgroundColor: '#BBB',
        }}
      >
        <CardContent>
          <Stack direction="row" spacing={4}>
            {data.smartestPet != null ? (
              <LoadableFieldReader loadableField={data.smartestPet} args={{}}>
                {(smartestPet) => (
                  <>
                    <smartestPet.Avatar
                      onClick={() =>
                        navigateTo({
                          kind: 'PetDetail',
                          id: smartestPet.id,
                        })
                      }
                    />
                    <div style={{ width: 300 }}>
                      <h2>#1: {smartestPet.fullName}</h2>
                      <p>
                        Intelligence level:{' '}
                        <b>{smartestPet.stats?.intelligence}</b>
                      </p>
                      <ul>
                        {smartestPet.checkinsPointer.length ? (
                          smartestPet.checkinsPointer.map(
                            (loadableCheckin, i) => {
                              return (
                                <li key={i}>
                                  <Suspense>
                                    <LoadableFieldReader
                                      loadableField={loadableCheckin}
                                      args={{}}
                                    >
                                      {(checkin) => (
                                        <>
                                          Checkin: <b>{checkin.location}</b>
                                        </>
                                      )}
                                    </LoadableFieldReader>
                                  </Suspense>
                                </li>
                              );
                            },
                          )
                        ) : (
                          <li>No checkins yet!</li>
                        )}
                      </ul>
                    </div>
                  </>
                )}
              </LoadableFieldReader>
            ) : (
              'No pets found!'
            )}
          </Stack>
        </CardContent>
      </Card>
    </Container>
  );
});

export const checkinsPointer = iso(`
  pointer Pet.checkinsPointer to [ICheckin!]! {
    checkins(
      limit: 2
    ) {
      __link
    }
  }
`)(({ data }) => {
  return data.checkins.map((checkin) => checkin.__link);
});

export const SmartestPet = iso(`
  pointer Query.smartestPet to Pet {
    pets {
      __link
      stats {
        intelligence
      }
    }
  }
`)(({ data }) => {
  let maxIntelligence = -Infinity;
  let maxIntelligenceLink = null;
  for (const pet of data.pets) {
    if ((pet.stats?.intelligence ?? 0) > maxIntelligence) {
      maxIntelligenceLink = pet.__link;
      maxIntelligence = pet.stats?.intelligence ?? 0;
    }
  }
  return maxIntelligenceLink;
});
