import React from 'react';
import { useResult, useLazyReference, subscribe } from '@isograph/react';

import { iso } from '@iso';
import { Container } from '@mui/material';

import { FullPageLoading, Route } from './GithubDemo';
import { RepoGitHubLink } from './RepoGitHubLink';

export const HomePage = iso(`
  field Query.HomePage($first: Int!) @component {
    Header
    HomePageList
  }
`)(function HomePageComponent({ data, route, setRoute }) {
  return (
    <>
      <data.Header route={route} setRoute={setRoute} />
      <Container maxWidth="md">
        <RepoGitHubLink filePath="demos/github-demo/src/isograph-components/HomeRoute.tsx">
          Home Page Route
        </RepoGitHubLink>
        <React.Suspense fallback={<FullPageLoading />}>
          <data.HomePageList route={route} setRoute={setRoute} />
        </React.Suspense>
      </Container>
    </>
  );
});

export function HomeRoute({
  route,
  setRoute,
}: {
  route: Route;
  setRoute: (route: Route) => void;
}) {
  const { queryReference } = useLazyReference(
    iso(`entrypoint Query.HomePage`),
    {
      first: 15,
    },
  );
  const Component = useResult(queryReference);
  return <Component route={route} setRoute={setRoute} />;
}
