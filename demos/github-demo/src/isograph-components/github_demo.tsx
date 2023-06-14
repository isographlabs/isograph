import React from "react";
import NoSSR from "react-no-ssr";
import { HomeRoute } from "./home";
import { RepositoryRoute } from "./repository";
import { Container } from "@mui/material";
import { UserRoute } from "./user";
import { PullRequestRoute } from "./pull_request";

export type Route =
  | {
      kind: "Home";
    }
  | RepositoryRoute
  | UserRoute
  | PullRequestRoute;

export type UserRoute = {
  kind: "User";
  userId: string;
  userLogin: string;
};

export type RepositoryRoute = {
  kind: "Repository";
  repositoryName: string;
  repositoryOwner: string;
  repositoryId: string;
};

export type PullRequestRoute = {
  kind: "PullRequest";
  pullRequestNumber: number;
  repositoryName: string;
  // this is owner login:
  repositoryOwner: string;
};

export function GithubDemo() {
  const [currentRoute, setCurrentRoute] = React.useState<Route>({
    // kind: "Home",
    kind: "Repository",
    repositoryName: "relay",
    repositoryOwner: "facebook",
    repositoryId: "123",
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

export function FullPageLoading() {
  return <h1 className="mt-5">Loading...</h1>;
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
      return <HomeRoute route={route} setRoute={setRoute} />;
    case "Repository":
      return (
        <RepositoryRoute
          route={route}
          setRoute={setRoute}
          key={route.repositoryId}
        />
      );
    case "User":
      return <UserRoute route={route} setRoute={setRoute} />;
    case "PullRequest":
      return <PullRequestRoute route={route} setRoute={setRoute} />;
    default:
      const exhaustiveCheck: never = route;
  }
}
