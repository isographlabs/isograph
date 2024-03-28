import React from 'react';
import { HomeRoute } from './HomeRoute';
import { RepositoryRoute } from './RepositoryRoute';
import { Container } from '@mui/material';
import { UserRoute } from './UserRoute';
import { PullRequestRoute } from './PullRequestRoute';

export type Route =
  | {
      kind: 'Home';
    }
  | RepositoryRoute
  | UserRoute
  | PullRequestRoute;

export type UserRoute = {
  kind: 'User';
  userLogin: string;
};

export type RepositoryRoute = {
  kind: 'Repository';
  repositoryName: string;
  repositoryOwner: string;
  repositoryId: string;
};

export type PullRequestRoute = {
  kind: 'PullRequest';
  pullRequestNumber: number;
  repositoryName: string;
  // this is owner login:
  repositoryOwner: string;
};

export function GithubDemo() {
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
  switch (route.kind) {
    case 'Home':
      return <HomeRoute route={route} setRoute={setRoute} />;
    case 'Repository':
      return (
        <RepositoryRoute
          route={route}
          setRoute={setRoute}
          key={route.repositoryId}
        />
      );
    case 'User':
      return <UserRoute route={route} setRoute={setRoute} />;
    case 'PullRequest':
      return <PullRequestRoute route={route} setRoute={setRoute} />;
    default:
      const _exhaustiveCheck: never = route;
      throw new Error('Unexpected route kind');
  }
}
