import { iso } from "@isograph/react";
import type { ResolverParameterType as HomePageListParams } from "./__isograph/Query/home_page_list.isograph";
import { RepoLink } from "./RepoLink";

export const home_page_list = iso<
  HomePageListParams,
  ReturnType<typeof HomePageList>
>`
  Query.home_page_list @component {
    viewer {
      login,
      id,
      name,
      repository_list,
    },
  }
`(HomePageList);

function HomePageList(props: HomePageListParams) {
  return (
    <>
      <RepoLink filePath="demos/github-demo/src/isograph-components/home_page_list.tsx">
        Home Page List Component
      </RepoLink>
      <h1>rbalicki2's repository stats</h1>
      {props.data.viewer.repository_list({ setRoute: props.setRoute })}
    </>
  );
}
