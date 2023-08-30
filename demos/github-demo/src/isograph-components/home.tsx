import React, { useEffect, useState } from "react";
import { iso, read, useLazyReference, subscribe } from "@isograph/react";
import { Container } from "@mui/material";

import homePageQuery from "./__isograph/Query/home_page.isograph";
import { FullPageLoading, Route } from "./github_demo";
import { RepoLink } from "./RepoLink";

iso`
  Query.home_page($first: Int!) @fetchable {
    header,
    home_page_list,
  }
`;

export function HomeRoute({
  route,
  setRoute,
}: {
  route: Route;
  setRoute: (route: Route) => void;
}) {
  const [, setState] = useState();
  useEffect(() => {
    return subscribe(() => setState({}));
  });
  const { queryReference } = useLazyReference(homePageQuery, {
    first: 15,
  });
  const data = read(queryReference);
  return (
    <>
      {data.header({ route, setRoute })}
      <Container maxWidth="md">
        <RepoLink filePath="demos/github-demo/src/isograph-components/home.tsx">
          Home Page Route
        </RepoLink>
        <React.Suspense fallback={<FullPageLoading />}>
          {data.home_page_list({
            setRoute,
          })}
        </React.Suspense>
      </Container>
    </>
  );
}
