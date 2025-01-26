import { iso } from '@iso';
import { useLazyReference, useResult } from '@isograph/react';
import { Container } from '@mui/material';
import React from 'react';
import {
  FullPageLoading,
  Route,
  type PullRequestRoute as PullRequestRouteType,
} from './GithubDemo';

export const PullRequest = iso(`
  field Query.PullRequest($repositoryOwner: String!, $repositoryName: String!, $pullRequestNumber: Int!) @component {
    Header
    PullRequestDetail(repositoryOwner: $repositoryOwner, repositoryName: $repositoryName, pullRequestNumber: $pullRequestNumber)
  }
`)(function PullRequestComponentComponent(
  { data },
  {
    route,
    setRoute,
  }: {
    route: Route;
    setRoute: (route: Route) => void;
  },
) {
  return (
    <>
      <data.Header route={route} setRoute={setRoute} />
      <Container maxWidth="md">
        <React.Suspense fallback={<FullPageLoading />}>
          <data.PullRequestDetail />
        </React.Suspense>
      </Container>
    </>
  );
});

export function PullRequestRoute({
  route,
  setRoute,
}: {
  route: PullRequestRouteType;
  setRoute: (route: Route) => void;
}) {
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Query.PullRequest`),
    {
      pullRequestNumber: route.pullRequestNumber,
      repositoryName: route.repositoryName,
      repositoryOwner: route.repositoryOwner,
    },
  );

  const Component = useResult(fragmentReference, {});
  return <Component route={route} setRoute={setRoute} />;
}
