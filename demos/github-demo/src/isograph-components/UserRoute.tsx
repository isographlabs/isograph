import React from 'react';
import { useLazyReference, useResult } from '@isograph/react';
import { iso } from '@iso';
import { Container } from '@mui/material';

import {
  FullPageLoading,
  Route,
  type UserRoute as UserRouteType,
} from './GithubDemo';

export const UserPage = iso(`
  field Query.UserPage($userLogin: String!) @component {
    Header
    UserDetail(userLogin: $userLogin)
  }
`)(function UserRouteComponentComponent(
  { data },
  {
    route,
    setRoute,
  }: {
    route: Route;
    setRoute: (newRoute: Route) => void;
  },
) {
  return (
    <>
      <data.Header route={route} setRoute={setRoute} />
      <Container maxWidth="md">
        <React.Suspense fallback={<FullPageLoading />}>
          <data.UserDetail setRoute={setRoute} />
        </React.Suspense>
      </Container>
    </>
  );
});

export function UserRoute({
  route,
  setRoute,
}: {
  route: UserRouteType;
  setRoute: (route: Route) => void;
}) {
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Query.UserPage`),
    {
      userLogin: route.userLogin,
      first: 20,
    },
  );
  const Component = useResult(fragmentReference, {});
  return <Component route={route} setRoute={setRoute} />;
}
