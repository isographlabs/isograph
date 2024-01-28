import React from "react";
import { iso, read, useLazyReference } from "@isograph/react";
import { Container } from "@mui/material";

import { ResolverParameterType as UserRouteComponentParams } from "@iso/Query/user_page/reader.isograph";
import {
  FullPageLoading,
  Route,
  type UserRoute as UserRouteType,
} from "./github_demo";

export const user_page = iso<
  UserRouteComponentParams,
  ReturnType<typeof UserRouteComponent>
>`
  Query.user_page($first: Int!, $userLogin: String!) @component {
    header,
    user_detail,
  }
`(UserRouteComponent);

function UserRouteComponent({
  data,
  route,
  setRoute,
}: UserRouteComponentParams) {
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

export function UserRoute({
  route,
  setRoute,
}: {
  route: UserRouteType;
  setRoute: (route: Route) => void;
}) {
  const { queryReference } = useLazyReference(iso`entrypoint Query.user_page`, {
    userLogin: route.userLogin,
    first: 20,
  });
  const component = read(queryReference);
  return component({ route, setRoute });
}
