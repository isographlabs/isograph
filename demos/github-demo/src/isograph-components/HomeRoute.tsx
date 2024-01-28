import React, { useEffect, useState } from "react";
import {
  iso,
  read,
  useLazyReference,
  subscribe,
  isoFetch,
} from "@isograph/react";
import { Container } from "@mui/material";

import { ResolverParameterType as HomePageComponentParams } from "@iso/Query/HomePage/reader.isograph";

import { FullPageLoading, Route } from "./GithubDemo";
import { RepoLink } from "./RepoLink";

export const HomePage = iso<
  HomePageComponentParams,
  ReturnType<typeof HomePageComponent>
>`
  Query.HomePage($first: Int!) @component {
    Header,
    HomePageList,
  }
`(HomePageComponent);

function HomePageComponent({ data, route, setRoute }: HomePageComponentParams) {
  return (
    <>
      <data.Header route={route} setRoute={setRoute} />
      <Container maxWidth="md">
        <RepoLink filePath="demos/github-demo/src/isograph-components/HomeRoute.tsx">
          Home Page Route
        </RepoLink>
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
  const { queryReference } = useLazyReference(isoFetch`Query.HomePage`, {
    first: 15,
  });
  const component = read(queryReference);
  return component({ route, setRoute });
}
