import React from "react";
import { bDeclare, useLazyReference, read } from "@isograph/react";
import { FullPageLoading, PullRequestRoute, Route } from "./github_demo";

import PullRequestQuery from "./__isograph/Query__pull_request.isograph";

import { Container } from "@mui/material";

bDeclare`
  Query.pull_request($repositoryOwner: String!, $repositoryName: String!, $pullRequestNumber: Int!, $last: Int!,) @fetchable {
    header,
    pull_request_detail,
  }
`;

export function PullRequestRoute({
  route,
  setRoute,
}: {
  route: PullRequestRoute;
  setRoute: (route: Route) => void;
}) {
  const { queryReference } = useLazyReference(PullRequestQuery, {
    pullRequestNumber: route.pullRequestNumber,
    repositoryName: route.repositoryName,
    repositoryOwner: route.repositoryOwner,
    last: 20,
  });

  const data = read(queryReference);

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
