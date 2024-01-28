import React from "react";
import { iso, useLazyReference, read, iso } from "@isograph/react";
import {
  FullPageLoading,
  type PullRequestRoute as PullRequestRouteType,
  Route,
} from "./github_demo";

import { ResolverParameterType as PullRequestComponentProps } from "@iso/Query/pull_request/reader.isograph";

import { Container } from "@mui/material";

export const pull_request = iso<
  PullRequestComponentProps,
  ReturnType<typeof PullRequestComponent>
>`
  field Query.pull_request($repositoryOwner: String!, $repositoryName: String!, $pullRequestNumber: Int!, $last: Int!) @component {
    header,
    pull_request_detail,
  }
`(PullRequestComponent);

function PullRequestComponent({
  data,
  route,
  setRoute,
}: PullRequestComponentProps) {
  return (
    <>
      {data.header({ route, setRoute })}
      <Container maxWidth="md">
        <React.Suspense fallback={<FullPageLoading />}>
          {data.pull_request_detail({ route, setRoute })}
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
  const { queryReference } = useLazyReference(iso`entrypoint Query.pull_request`, {
    pullRequestNumber: route.pullRequestNumber,
    repositoryName: route.repositoryName,
    repositoryOwner: route.repositoryOwner,
    last: 20,
  });

  const data = read(queryReference);
  return data({ route, setRoute });
}
