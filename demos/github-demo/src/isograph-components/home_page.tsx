import React, { useEffect, useState } from "react";
import {
  iso,
  read,
  useLazyReference,
  subscribe,
  iso,
} from "@isograph/react";
import { Container } from "@mui/material";

import { ResolverParameterType as HomePageComponentParams } from "@iso/Query/home_page_component/reader.isograph";

import { FullPageLoading, Route } from "./github_demo";
import { RepoLink } from "./RepoLink";

export const home_page_component = iso<
  HomePageComponentParams,
  ReturnType<typeof HomePageComponent>
>`
  Query.home_page_component($first: Int!) @component {
    header,
    home_page_list,
  }
`(HomePageComponent);

function HomePageComponent({ data, route, setRoute }: HomePageComponentParams) {
  return (
    <>
      {data.header({ route, setRoute })}
      <Container maxWidth="md">
        <RepoLink filePath="demos/github-demo/src/isograph-components/home.tsx">
          Home Page Route
        </RepoLink>
        {/* <div
          onClick={() =>
            data.home_page_list_wrapper.viewer?.status?.__update_user_bio({
              bio: "asdf",
            })
          }
        >
          Update user bio
        </div> */}
        <React.Suspense fallback={<FullPageLoading />}>
          {data.home_page_list({ route, setRoute })}
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
  const { queryReference } = useLazyReference(
    iso`entrypoint Query.home_page_component`,
    {
      first: 15,
    }
  );
  const component = read(queryReference);
  return component({ route, setRoute });
}
