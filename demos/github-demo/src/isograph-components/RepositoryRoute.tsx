import { iso } from '@iso';
import { useLazyReference, useResult } from '@isograph/react';
import { Container } from '@mui/material';
import React from 'react';
import {
  FullPageLoading,
  RepositoryRoute as RepositoryRouteType,
  Route,
} from './GithubDemo';

export const RepositoryPage = iso(`
  field Query.RepositoryPage($repositoryName: String!, $repositoryOwner: String!, $first: Int!) @component {
    Header
    RepositoryDetail(repositoryName: $repositoryName, repositoryOwner: $repositoryOwner, first: $first)
  }
`)(function RepositoryRouteComponent(
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
          <data.RepositoryDetail setRoute={setRoute} />
        </React.Suspense>
      </Container>
    </>
  );
});

export function RepositoryRoute({
  route,
  setRoute,
}: {
  route: RepositoryRouteType;
  setRoute: (route: Route) => void;
}) {
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Query.RepositoryPage`),
    {
      repositoryName: route.repositoryName,
      repositoryOwner: route.repositoryOwner,
      first: 20,
    },
  );
  const Component = useResult(fragmentReference, {});
  return <Component route={route} setRoute={setRoute} />;
}
