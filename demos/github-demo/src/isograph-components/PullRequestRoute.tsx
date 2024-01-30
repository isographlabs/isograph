import React from 'react';
import { iso, useLazyReference, read } from '@isograph/react';
import {
  FullPageLoading,
  type PullRequestRoute as PullRequestRouteType,
  Route,
} from './GithubDemo';

import { ResolverParameterType as PullRequestComponentProps } from '@iso/Query/PullRequest/reader.isograph';

import { Container } from '@mui/material';

export const PullRequest = iso<
  PullRequestComponentProps,
  ReturnType<typeof PullRequestComponentComponent>
>`
  field Query.PullRequest($repositoryOwner: String!, $repositoryName: String!, $pullRequestNumber: Int!, $last: Int!) @component {
    Header,
    PullRequestDetail,
  }
`(PullRequestComponentComponent);

function PullRequestComponentComponent({ data, route, setRoute }: PullRequestComponentProps) {
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
}

export function PullRequestRoute({
  route,
  setRoute,
}: {
  route: PullRequestRouteType;
  setRoute: (route: Route) => void;
}) {
  const { queryReference } = useLazyReference(iso`entrypoint Query.PullRequest `, {
    pullRequestNumber: route.pullRequestNumber,
    repositoryName: route.repositoryName,
    repositoryOwner: route.repositoryOwner,
    last: 20,
  });

  const data = read(queryReference);
  return data({ route, setRoute });
}
