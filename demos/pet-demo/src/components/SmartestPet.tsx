import { LoadableFieldReader } from '@isograph/react';
import { Card, CardContent, Container, Stack } from '@mui/material';
import { Suspense } from 'react';
import { iso } from '@iso';
import { useNavigateTo } from './routes';

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
      firstCheckin {
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
                      <p>
                        {smartestPet.firstCheckin ? (
                          <Suspense>
                            <LoadableFieldReader
                              loadableField={smartestPet.firstCheckin}
                              args={{}}
                            >
                              {(firstCheckin) => (
                                <>
                                  Next checkin location:{' '}
                                  <b>{firstCheckin.location}</b>
                                </>
                              )}
                            </LoadableFieldReader>
                          </Suspense>
                        ) : (
                          'No checkins yet!'
                        )}
                      </p>
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

export const firstCheckin = iso(`
  pointer Pet.firstCheckin to ICheckin {
    checkins(limit: 1) {
      __link
    }
  }
`)(({ data }) => {
  return data.checkins[0].__link ?? null;
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
