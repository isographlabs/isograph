import React from "react";
import NoSSR from "react-no-ssr";
import { Container } from "@mui/material";
import { HomeRoute } from "./home_route";
import { PetDetailRoute } from "./pet_detail_route";
import { subscribe } from "@isograph/react";

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
  console.log({ route });
  switch (route.kind) {
    case "Home":
      return <HomeRoute navigateTo={setRoute} />;
    case "PetDetail":
      return (
        <PetDetailRoute navigateTo={setRoute} route={route} key={route.id} />
      );
    default:
      const exhaustiveCheck: never = route;
  }
}

export function FullPageLoading() {
  return <h1 className="mt-5">Loading...</h1>;
}
