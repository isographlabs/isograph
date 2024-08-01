import React from 'react';
import { iso } from '@iso';
import { Container, Stack } from '@mui/material';
import {
  EntrypointReader,
  useClientSideDefer,
  useLazyReference,
  useResult,
} from '@isograph/react';
import { PetDetailDeferredRoute, useNavigateTo } from './routes';

export const PetDetailDeferredRouteComponent = iso(`
  field Query.PetDetailDeferredRoute($id: ID!) @component {
    pet(id: $id) {
      PetDetailDeferredRouteInnerComponent
    }
  }
`)(function PetDetailRouteComponent(data) {
  const { pet } = data;
  if (pet == null) {
    return <h1>Pet not found.</h1>;
  }

  return <pet.PetDetailDeferredRouteInnerComponent />;
});

export const PetDetailDeferredRouteInnerComponent = iso(`
  field Pet.PetDetailDeferredRouteInnerComponent @component {
    name
    PetCheckinsCard @loadable
  }
`)((pet) => {
  const navigateTo = useNavigateTo();
  // @ts-expect-error
  const petCheckinsCard = useClientSideDefer(pet.PetCheckinsCard, {
    count: 2,
  });

  return (
    <Container maxWidth="md">
      <h1>Pet Detail for {pet.name}</h1>
      <h3
        onClick={() => navigateTo({ kind: 'Home' })}
        style={{ cursor: 'pointer' }}
      >
        ← Home
      </h3>
      <React.Suspense fallback={<h2>Loading pet details...</h2>}>
        <Stack direction="row" spacing={4}>
          <Stack direction="column" spacing={4}>
            <EntrypointReader queryReference={petCheckinsCard} />
          </Stack>
        </Stack>
      </React.Suspense>
    </Container>
  );
});

export function PetDetailDeferredRouteLoader({
  route,
}: {
  route: PetDetailDeferredRoute;
}) {
  const { queryReference } = useLazyReference(
    iso(`entrypoint Query.PetDetailDeferredRoute`),
    { id: route.id },
  );

  const Component = useResult(queryReference);
  return <Component />;
}
