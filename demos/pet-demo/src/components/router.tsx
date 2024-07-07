import React from 'react';
import { Container } from '@mui/material';
import { useLazyReference, useResult } from '@isograph/react';
import { iso } from '@iso';
import { useRouter } from 'next/router';

export type PetId = string;

export type Route = HomeRoute | PetDetailRoute | LoadableRoute;

export type HomeRoute = {
  kind: 'Home';
};

export type PetDetailRoute = {
  kind: 'PetDetail';
  id: PetId;
};

export type LoadableRoute = {
  kind: 'Loadable';
};

function toRoute(route: Route): string {
  switch (route.kind) {
    case 'Home': {
      return '/';
    }
    case 'PetDetail': {
      return `/pet/${route.id}`;
    }
    case 'Loadable': {
      return '/loadable';
    }
  }
}

export function GraphQLConfDemo(props: { initialState: Route }) {
  const router = useRouter();
  const [currentRoute, setCurrentRoute] = React.useState<Route>(
    props.initialState,
  );

  const updateRoute = (route: Route) => {
    setCurrentRoute(route);
    router.push(toRoute(route));
  };

  return (
    <React.Suspense
      fallback={
        <Container maxWidth="md">
          <FullPageLoading />
        </Container>
      }
    >
      <Router route={currentRoute} setRoute={updateRoute} />
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
    case 'Loadable':
      return <LoadableDemo />;
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

// If window.__LOG is true, Isograph will log a bunch of diagnostics.
if (typeof window !== 'undefined') {
  window.__LOG = true;
}

function LoadableDemo() {
  const { queryReference } = useLazyReference(
    iso(`entrypoint Query.LoadableDemo`),
    {},
  );

  const Component = useResult(queryReference);
  if (typeof window !== 'undefined') {
    // @ts-expect-error
    window.data = Component;
  }
  return <Component />;
}
