import React from 'react';
import { useLazyReference, useResult } from '@isograph/react';
import { iso } from '@iso';
import {
  FullPageLoading,
  type PullRequestRoute as PullRequestRouteType,
  Route,
} from './GithubDemo';

import { Container } from '@mui/material';

export const PullRequest = iso(`
  field Query.PullRequest($repositoryOwner: String!, $repositoryName: String!, $pullRequestNumber: Int!, $last: Int!) @component {
    Header
    PullRequestDetail
  }
`)(function PullRequestComponentComponent({ data, route, setRoute }) {
  return (
    <>
      <data.Header route={route} setRoute={setRoute} />
      <Container maxWidth="md">
        <React.Suspense fallback={<FullPageLoading />}>
          <data.PullRequestDetail route={route} setRoute={setRoute} />
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
  const { queryReference } = useLazyReference(
    iso(`entrypoint Query.PullRequest`),
    {
      pullRequestNumber: route.pullRequestNumber,
      repositoryName: route.repositoryName,
      repositoryOwner: route.repositoryOwner,
      last: 20,
    },
  );

  const Component = useResult(queryReference);
  return <Component route={route} setRoute={setRoute} />;
}
