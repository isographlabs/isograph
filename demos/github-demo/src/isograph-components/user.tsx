import React from "react";
import { iso, read, useLazyReference } from "@isograph/react";
import { Container } from "@mui/material";

import userPageQuery from "./__isograph/Query__user_page.isograph";
import { FullPageLoading, Route, UserRoute } from "./github_demo";

iso`
  Query.user_page($first: Int!, $userLogin: String!) @fetchable {
    header,
    user_detail,
  }
`;

export function UserRoute({
  route,
  setRoute,
}: {
  route: UserRoute;
  setRoute: (route: Route) => void;
}) {
  const { queryReference } = useLazyReference(userPageQuery, {
    userLogin: route.userLogin,
    first: 20,
  });
  const data = read(queryReference);
  return (
    <>
      {data.header({ route, setRoute })}
      <Container maxWidth="md">
        <React.Suspense fallback={<FullPageLoading />}>
          {data.user_detail({
            setRoute,
          })}
        </React.Suspense>
      </Container>
    </>
  );
}
