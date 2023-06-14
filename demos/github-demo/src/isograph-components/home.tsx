import React from "react";
import { bDeclare, read, useLazyReference } from "@isograph/react";
import { Container } from "@mui/material";

import homePageQuery from "./__isograph/Query__home_page.isograph";
import { FullPageLoading, Route } from "./github_demo";

bDeclare`
  Query.home_page($first: Int!,) @fetchable {
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
  const { queryReference } = useLazyReference(homePageQuery, { first: 15 });
  const data = read(queryReference);
  return (
    <>
      {data.header({ route, setRoute })}
      <Container maxWidth="md">
        <React.Suspense fallback={<FullPageLoading />}>
          {data.home_page_list({
            setRoute,
          })}
        </React.Suspense>
      </Container>
    </>
  );
}
