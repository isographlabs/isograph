import { iso } from './__isograph/iso';

export const SmartestPetRoute = iso(`
  field Query.SmartestPetRoute @component {
    smartestPet {
      name
    }
  }
`)(function SmartestRouteComponent({ data }) {
  return <div>smartest yaya</div>;
  return (
    <Container maxWidth="md">
      <h1>Robert&apos;s Pet List 3000</h1>
      <Stack direction="column" spacing={4}>
        {data.pets.map((pet) => (
          <pet.PetSummaryCard key={pet.id} />
        ))}
      </Stack>
    </Container>
  );
});

export const SmartestPet = iso(`
  pointer Query.smartestPet to Pet {
    pets {
      id
      stats {
        intelligence
      }
    }
  }
`)(({ data }) => {
  return data.pets[0].id;
});
