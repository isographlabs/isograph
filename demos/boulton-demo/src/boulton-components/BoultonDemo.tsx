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
  const { queryReference } = useLazyReference(userListPageQuery, {
    bar: "yayayaya",
    // bar2: "bar2",
  });
  // TODO get this to work:
  // const {queryReference} = useLazyReference(b Declare ` ... `);
  console.log("queryReference", queryReference);

  const listPageData = read(queryReference);
  console.log("listPageData", listPageData);
  return listPageData;

  return (
    <>
      <h1>Users</h1>
      {listPageData.byah}
      {listPageData.users.map((user) => {
        const user_component = user.user_list_component({ onSelectId });
        return <div key={user.id}>{user_component}</div>;
      })}
    </>
  );
}

function TopLevelUserProfileWithDetails({ onGoBack }) {
  // TODO replace this with the trick that causes graphql`...` literals to work
  const { queryReference } = useLazyReference(userDetailPageQuery, {});
  const data = read(queryReference);
  return data.current_user.user_profile_with_details({ onGoBack });
}

function FullPageLoading() {
  return <h1 className="mt-5">Loading...</h1>;
}
