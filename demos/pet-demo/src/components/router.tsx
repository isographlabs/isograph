import React from 'react';
import { Container } from '@mui/material';
import { useLazyReference, useResult } from '@isograph/react';
import { iso } from '@iso';

export type PetId = string;

export type Route = HomeRoute | PetDetailRoute;

export type HomeRoute = {
  kind: 'Home';
};

export type PetDetailRoute = {
  kind: 'PetDetail';
  id: PetId;
};

export function GraphQLConfDemo(props: {}) {
  const [currentRoute, setCurrentRoute] = React.useState<Route>({
    kind: 'Home',
  });
  return (
    <React.Suspense
      fallback={
        <Container maxWidth="md">
          <FullPageLoading />
        </Container>
      }
    >
      <Router route={currentRoute} setRoute={setCurrentRoute} />
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

if (typeof window !== 'undefined') {
  // @ts-expect-error
  window.__LOG = true;
}
