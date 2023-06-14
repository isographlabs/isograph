import { iso } from "@isograph/react";
import type { ResolverParameterType as HomePageListParams } from "./__isograph/Query__home_page_list.isograph";

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
      <h1>Your repository stats</h1>
      {props.data.viewer.repository_list({ setRoute: props.setRoute })}
    </>
  );
}
