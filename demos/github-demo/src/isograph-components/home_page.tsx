import React, { useEffect, useState } from "react";
import { iso, read, useLazyReference, subscribe, isoFetch } from "@isograph/react";
import { Container } from "@mui/material";

import homePageQuery from "./__isograph/Query/home_page/entrypoint.isograph";
import { FullPageLoading, Route } from "./github_demo";
import { RepoLink } from "./RepoLink";

iso`
  Query.home_page($first: Int!) @fetchable {
    header,
    home_page_list_wrapper,
  }
`;

isoFetch`
  Query.home_page
`

// home_page_list_wrapper exists solely to test some __refetch things.
export const home_page_list_wrapper = iso`
  Query.home_page_list_wrapper($first: Int!) @eager {
    home_page_list,
    viewer {
      status {
        emoji,
        __update_user_bio,
        user {
          id,
          repositories(last: $first) {
            __typename,
          },
        },
      },
    },
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
        <div
          onClick={() =>
            data.home_page_list_wrapper.viewer?.status?.__update_user_bio({
              bio: "asdf",
            })
          }
        >
          Update user bio
        </div>
        <React.Suspense fallback={<FullPageLoading />}>
          {data.home_page_list_wrapper.home_page_list({})}
        </React.Suspense>
      </Container>
    </>
  );
}
