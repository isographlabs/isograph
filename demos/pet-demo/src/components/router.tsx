import React from 'react';
import { Container } from '@mui/material';
import { useLazyReference, useResult } from '@isograph/react';
import { iso } from '@iso';
import { useRouter } from 'next/router';

export type PetId = string;

export type Route = HomeRoute | PetDetailRoute | PetDetailDeferredRoute;

export type HomeRoute = {
  kind: 'Home';
};

export type PetDetailRoute = {
  kind: 'PetDetail';
  id: PetId;
};

export type PetDetailDeferredRoute = {
  kind: 'PetDetailDeferred';
  id: PetId;
};

function toRoute(route: Route): string {
  switch (route.kind) {
    case 'Home': {
      return '/';
    }
    case 'PetDetail': {
      return `/pet/${route.id}`;
    }
    case 'PetDetailDeferred': {
      return `/pet/with-defer/${route.id}`;
    }
  }
}

export function GraphQLConfDemo({ route }: { route: Route }) {
  const router = useRouter();

  const updateRoute = (newRoute: Route) => {
    router.push(toRoute(newRoute));
  };

  return (
    <React.Suspense
      fallback={
        <Container maxWidth="md">
          <FullPageLoading />
        </Container>
      }
    >
      <Router route={route} setRoute={updateRoute} />
    </React.Suspense>
  );
}

function Router({
  route,
  setRoute,
}: {
  route: Route;
  setRoute: (route: Route) => void;
}) {
  switch (route.kind) {
    case 'Home':
      return <HomeRouteLoader navigateTo={setRoute} />;
    case 'PetDetail':
      return <PetDetailRouteLoader navigateTo={setRoute} route={route} />;
    case 'PetDetailDeferred':
      return (
        <PetDetailDeferredRouteLoader navigateTo={setRoute} route={route} />
      );
    default:
      const exhaustiveCheck: never = route;
  }
}

export function FullPageLoading() {
  return <h1 className="mt-5">Loading...</h1>;
}

function HomeRouteLoader({
  navigateTo,
}: {
  navigateTo: (path: Route) => void;
}) {
  const { queryReference } = useLazyReference(
    iso(`entrypoint Query.HomeRoute`),
    {},
  );

  const Component = useResult(queryReference);
  return <Component navigateTo={navigateTo} />;
}

function PetDetailRouteLoader({
  navigateTo,
  route,
}: {
  navigateTo: (path: Route) => void;
  route: PetDetailRoute;
}) {
  const { queryReference } = useLazyReference(
    iso(`entrypoint Query.PetDetailRoute`),
    { id: route.id },
  );

  const Component = useResult(queryReference);
  return <Component navigateTo={navigateTo} />;
}

function PetDetailDeferredRouteLoader({
  navigateTo,
  route,
}: {
  navigateTo: (path: Route) => void;
  route: PetDetailDeferredRoute;
}) {
  const { queryReference } = useLazyReference(
    iso(`entrypoint Query.PetDetailDeferredRoute`),
    { id: route.id },
  );

  const Component = useResult(queryReference);
  return <Component navigateTo={navigateTo} />;
}

// If window.__LOG is true, Isograph will log a bunch of diagnostics.
if (typeof window !== 'undefined') {
  window.__LOG = true;
}
