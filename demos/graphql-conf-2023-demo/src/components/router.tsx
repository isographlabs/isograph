import React from "react";
import NoSSR from "react-no-ssr";
import { Container } from "@mui/material";
import { subscribe, isoFetch, useLazyReference, read } from "@isograph/react";
import HomeRouteEntrypoint from "@iso/Query/home_route/entrypoint.isograph";
import PetDetailRouteEntrypoint from "@iso/Query/pet_detail_route/entrypoint.isograph";

export type PetId = string;

export type Route = HomeRoute | PetDetailRoute;

export type HomeRoute = {
  kind: "Home";
};

export type PetDetailRoute = {
  kind: "PetDetail";
  id: PetId;
};

export function GraphQLConfDemo(props: {}) {
  // N.B. we are rerendering the root component on any store change
  // here. Isograph will support more fine-grained re-rendering in
  // the future, and this will be done automatically as part of
  // useLazyReference.
  const [, setState] = React.useState<object | void>();
  React.useEffect(() => {
    return subscribe(() => setState({}));
  });

  const [currentRoute, setCurrentRoute] = React.useState<Route>({
    kind: "Home",
  });
  return (
    <NoSSR>
      <React.Suspense
        fallback={
          <Container maxWidth="md">
            <FullPageLoading />
          </Container>
        }
      >
        <Router route={currentRoute} setRoute={setCurrentRoute} />
      </React.Suspense>
    </NoSSR>
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
    case "Home":
      return <HomeRouteLoader navigateTo={setRoute} />;
    case "PetDetail":
      return (
        <PetDetailRouteLoader
          navigateTo={setRoute}
          route={route}
          key={route.id}
        />
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
    isoFetch<typeof HomeRouteEntrypoint>`
      Query.home_route
    `,
    {}
  );

  return read(queryReference)({ navigateTo });
}

function PetDetailRouteLoader({
  navigateTo,
  route,
}: {
  navigateTo: (path: Route) => void;
  route: PetDetailRoute;
}) {
  const { queryReference } = useLazyReference(
    isoFetch<typeof PetDetailRouteEntrypoint>`
      Query.pet_detail_route
    `,
    { id: route.id }
  );

  return read(queryReference)({ navigateTo });
}
