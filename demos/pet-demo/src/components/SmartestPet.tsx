import { LoadableFieldReader } from '@isograph/react';
import { Card, CardContent, Container, Stack } from '@mui/material';
import { iso } from './__isograph/iso';
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
                      Intelligence level:{' '}
                      <b>{smartestPet.stats?.intelligence}</b>
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
      link
    }
  }
`)(({ data }) => {
  return data.checkins[0].link ?? null;
});

export const SmartestPet = iso(`
  pointer Query.smartestPet to Pet {
    pets {
      link
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
      maxIntelligenceLink = pet.link;
      maxIntelligence = pet.stats?.intelligence ?? 0;
    }
  }
  return maxIntelligenceLink;
});
