import React from 'react';
import { useLazyReference, useResult } from '@isograph/react';
import { iso } from '@iso';
import { Container } from '@mui/material';

import {
  FullPageLoading,
  Route,
  RepositoryRoute as RepositoryRouteType,
} from './GithubDemo';

export const RepositoryPage = iso(`
  field Query.RepositoryPage($repositoryName: String!, $repositoryOwner: String!, $first: Int!) @component {
    Header
    RepositoryDetail
  }
`)(function RepositoryRouteComponent(
  data,
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
          <data.RepositoryDetail setRoute={setRoute} />
        </React.Suspense>
      </Container>
    </>
  );
});

if (typeof window !== 'undefined') {
  window.__LOG = true;
}

export function RepositoryRoute({
  route,
  setRoute,
}: {
  route: RepositoryRouteType;
  setRoute: (route: Route) => void;
}) {
  const { queryReference } = useLazyReference(
    iso(`entrypoint Query.RepositoryPage`),
    {
      repositoryName: route.repositoryName,
      repositoryOwner: route.repositoryOwner,
      first: 20,
    },
  );
  const Component = useResult(queryReference);
  return <Component route={route} setRoute={setRoute} />;
}
