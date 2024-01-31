import React from 'react';
import { iso, read, useLazyReference } from '@isograph/react';
import { Container } from '@mui/material';
import { ResolverParameterType as RepositoryPageParams } from '@iso/Query/RepositoryPage/reader.isograph';
import RepositoryPageEntrypoint from '@iso/Query/RepositoryPage/entrypoint.isograph';

import { FullPageLoading, Route, RepositoryRoute as RepositoryRouteType } from './GithubDemo';

export const RepositoryPage = iso(`
  field Query.RepositoryPage(
    $repositoryName: String!,
    $repositoryOwner: String!,
    $first: Int!
  ) @component {
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
    iso`entrypoint Query.RepositoryPage`,
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
  const Component = read(queryReference);
  return <Component route={route} setRoute={setRoute} />;
}
