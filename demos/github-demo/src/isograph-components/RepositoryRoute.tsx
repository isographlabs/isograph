import React from 'react';
import { useLazyReference, useRead } from '@isograph/react';
import { iso } from '@iso';
import { Container } from '@mui/material';
import RepositoryPageEntrypoint from '@iso/Query/RepositoryPage/entrypoint';

import {
  FullPageLoading,
  Route,
  RepositoryRoute as RepositoryRouteType,
} from './GithubDemo';

export const RepositoryPage = iso(`
  field Query.RepositoryPage($repositoryName: String!, $repositoryOwner: String!, $first: Int!) @component {
    Header,
    RepositoryDetail,
  }
`)(function RepositoryRouteComponent({ data, route, setRoute }) {
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
  const { queryReference } = useLazyReference<typeof RepositoryPageEntrypoint>(
    iso(`entrypoint Query.RepositoryPage`),
    {
      repositoryName: route.repositoryName,
      repositoryOwner: route.repositoryOwner,
      first: 20,
    },
  );
  console.log('repository route', {
    queryReference,
    name: route.repositoryName,
  });
  const Component = useRead(queryReference);
  return <Component route={route} setRoute={setRoute} />;
}
