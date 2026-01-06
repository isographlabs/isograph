import { iso } from '@iso';
import {
  FragmentRenderer,
  useClientSideDefer,
  useLazyReference,
} from '@isograph/react';
import { Container, Stack } from '@mui/material';
import React from 'react';
import { ErrorBoundary } from './ErrorBoundary';
import type { PetDetailDeferredRoute } from './routes';
import { FullPageLoading, useNavigateTo } from './routes';

export const PetDetailDeferredRouteComponent = iso(`
  field Query.PetDetailDeferredRoute(
    $id: ID !
  ) @component {
    pet(
      id: $id
    ) {
      PetDetailDeferredRouteInnerComponent
    }
    topLevelField(
      input: {
        name: "ThisIsJustHereToTestObjectLiterals"
      }
    ) {
      __typename
    }
    namable {
      __typename
    }
    notImplemented {
      __typename
      __link
    }
  }
`)(function PetDetailRouteComponent({ data }) {
  const { pet } = data;
  if (pet == null) {
    return <h1>Pet not found.</h1>;
  }

  return <pet.PetDetailDeferredRouteInnerComponent />;
});

// TODO split into separate files!
export const PetDetailDeferredRouteInnerComponent = iso(`
  field Pet.PetDetailDeferredRouteInnerComponent @component {
    fullName
    PetCheckinsCard @loadable
  }
`)(({ data: pet }) => {
  const navigateTo = useNavigateTo();
  const { fragmentReference: petCheckinsCard } = useClientSideDefer(
    pet.PetCheckinsCard,
    { limit: 2 },
  );

  return (
    <Container maxWidth="md">
      <h1>Pet Detail for {pet.fullName}</h1>
      <h3
        onClick={() => navigateTo({ kind: 'Home' })}
        style={{ cursor: 'pointer' }}
      >
        ← Home
      </h3>
      <React.Suspense fallback={<h2>Loading pet details...</h2>}>
        <Stack direction="row" spacing={4}>
          <Stack direction="column" spacing={4}>
            <FragmentRenderer fragmentReference={petCheckinsCard} />
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
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Query.PetDetailDeferredRoute`),
    { id: route.id },
  );

  return (
    <ErrorBoundary>
      <React.Suspense fallback={<FullPageLoading />}>
        <FragmentRenderer
          fragmentReference={fragmentReference}
          networkRequestOptions={{ suspendIfInFlight: false }}
        />
      </React.Suspense>
    </ErrorBoundary>
  );
}
