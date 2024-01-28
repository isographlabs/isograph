import React from "react";
import { iso, read, useLazyReference } from "@isograph/react";
import { Container } from "@mui/material";
import { ResolverParameterType as RepositoryPageParams } from "@iso/Query/repository_page/reader.isograph";

import {
  FullPageLoading,
  Route,
  RepositoryRoute as RepositoryRouteType,
} from "./github_demo";

export const repository_page = iso<
  RepositoryPageParams,
  ReturnType<typeof RepositoryRouteComponent>
>`
  field Query.repository_page($repositoryName: String!, $repositoryOwner: String!, $first: Int!) @component {
    header,
    repository_detail,
  }
`(RepositoryRouteComponent);

function RepositoryRouteComponent({
  data,
  route,
  setRoute,
}: RepositoryPageParams) {
  return (
    <>
      {data.header({ route, setRoute })}
      <Container maxWidth="md">
        <React.Suspense fallback={<FullPageLoading />}>
          {data.repository_detail({
            setRoute,
          })}
        </React.Suspense>
      </Container>
    </>
  );
}

export function RepositoryRoute({
  route,
  setRoute,
}: {
  route: RepositoryRouteType;
  setRoute: (route: Route) => void;
}) {
  const { queryReference } = useLazyReference(iso`entrypoint Query.repository_page`, {
    repositoryName: route.repositoryName,
    repositoryOwner: route.repositoryOwner,
    first: 20,
  });
  console.log("repository route", { queryReference });
  const data = read(queryReference);
  return data({ route, setRoute });
}
