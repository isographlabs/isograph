import React from 'react';
import { useResult, useLazyReference, subscribe } from '@isograph/react';

import { iso } from '@iso';
import { Container } from '@mui/material';

import { FullPageLoading, Route } from './GithubDemo';
import { RepoGitHubLink } from './RepoGitHubLink';

export const HomePage = iso(`
  field Query.HomePage @component {
    Header
    HomePageList
  }
`)(function HomePageComponent(
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
        <RepoGitHubLink filePath="demos/github-demo/src/isograph-components/HomeRoute.tsx">
          Home Page Route
        </RepoGitHubLink>
        <React.Suspense fallback={<FullPageLoading />}>
          <data.HomePageList setRoute={setRoute} />
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
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Query.HomePage`),
    {},
  );
  const Component = useResult(fragmentReference, {});
  return <Component route={route} setRoute={setRoute} />;
}
