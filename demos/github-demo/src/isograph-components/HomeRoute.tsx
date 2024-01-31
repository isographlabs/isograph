import React, { useEffect, useState } from 'react';
import { iso, useRead, useLazyReference, subscribe } from '@isograph/react';
import { Container } from '@mui/material';

import { ResolverParameterType as HomePageComponentParams } from '@iso/Query/HomePage/reader.isograph';
import HomePageEntrypoint from '@iso/Query/HomePage/entrypoint.isograph';

import { FullPageLoading, Route } from './GithubDemo';
import { RepoGitHubLink } from './RepoGitHubLink';

export const HomePage = iso<HomePageComponentParams>`
  field Query.HomePage($first: Int!) @component {
    Header,
    HomePageList,
  }
`(HomePageComponent);

function HomePageComponent({ data, route, setRoute }: HomePageComponentParams) {
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
}

export function HomeRoute({
  route,
  setRoute,
}: {
  route: Route;
  setRoute: (route: Route) => void;
}) {
  const [, setState] = useState({});
  useEffect(() => {
    return subscribe(() => setState({}));
  }, []);
  const { queryReference } = useLazyReference<typeof HomePageEntrypoint>(
    iso`entrypoint Query.HomePage`,
    {
      first: 15,
    },
  );
  const Component = useRead(queryReference);
  return <Component route={route} setRoute={setRoute} />;
}
