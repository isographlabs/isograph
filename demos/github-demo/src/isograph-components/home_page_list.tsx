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
      name,
      repository_list,
      __refetch,
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
      <div onClick={() => props.data.viewer.__refetch()}>refetch</div>
      {props.data.viewer.repository_list({ setRoute: props.setRoute })}
    </>
  );
}
