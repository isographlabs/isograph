import React from "react";
import NoSSR from "react-no-ssr";

import userListPageQuery from "./__boulton/Query__user_list_page.boulton";
import userDetailPageQuery from "./__boulton/Query__user_detail_page.boulton";

import { useLazyReference, read } from "@boulton/react";

export function BoultonDemo() {
  const [selectedId, setSelectedId] = React.useState<null | string>(null);
  return (
    <NoSSR>
      <React.Suspense fallback={<FullPageLoading />}>
        {selectedId ? (
          <TopLevelUserProfileWithDetails
            onGoBack={() => setSelectedId(null)}
          />
        ) : (
          <TopLevelListView onSelectId={(id: string) => setSelectedId(id)} />
        )}
      </React.Suspense>
    </NoSSR>
  );
}

function TopLevelListView({ onSelectId }) {
  // TODO replace this with the trick that causes graphql`...` literals to work
  const { queryReference } = useLazyReference(userListPageQuery);
  const listPageData = read(queryReference);

  return (
    <>
      <h1>Users</h1>
      {listPageData.users.map((user) => {
        const user_list_component = read(user.user_list_component);
        return (
          console.log("user", user_list_component) || (
            <div key={user.id}>{user_list_component(onSelectId)}</div>
          )
        );
      })}
    </>
  );
}

function TopLevelUserProfileWithDetails({ onGoBack }) {
  // TODO replace this with the trick that causes graphql`...` literals to work
  const { queryReference } = useLazyReference(userDetailPageQuery);
  const data = read(queryReference);
  const user_profile_with_details = read(
    data.current_user.user_profile_with_details
  );
  return user_profile_with_details(onGoBack);
}

function FullPageLoading() {
  return <h1 className="mt-5">Loading...</h1>;
}
