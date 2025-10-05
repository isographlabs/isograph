import { LoadableFieldReader } from '@isograph/react';
import { iso } from './__isograph/iso';
import { Card, CardContent, Container, Stack } from '@mui/material';
import { useNavigateTo } from './routes';

export const SmartestPetRoute = iso(`
  field Query.SmartestPetRoute @component {
    smartestPet {
      id
      name
      Avatar
      stats {
        intelligence
      }
      picture
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
                      <h2>#1: {smartestPet.name}</h2>
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
