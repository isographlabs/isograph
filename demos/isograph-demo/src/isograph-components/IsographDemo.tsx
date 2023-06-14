import React from "react";
import NoSSR from "react-no-ssr";

import userListPageQuery from "./__isograph/Query__user_list_page.isograph";
import userDetailPageQuery from "./__isograph/Query__user_detail_page.isograph";
import type { ResolverParameterType as QueryUserListPage } from "./__isograph/Query__user_list_page.isograph";

import { useLazyReference, read, iso } from "@isograph/react";

iso<QueryUserListPage>`
  Query.user_list_page(
    $bar: String!,
  ) @fetchable {
    byah(foo: $bar,),
    users {
      id,
      user_detail,
    },
  }
`;

export function IsographDemo() {
  const [selectedId, setSelectedId] = React.useState<null | string>(null);
  return (
    <NoSSR>
      <React.Suspense fallback={<FullPageLoading />}>
        {selectedId ? (
          <TopLevelUserProfileWithDetails
            selectedId={selectedId}
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
    bar: "baz",
  });

  const listPageData = read(queryReference);

  return (
    <>
      <h1>Users</h1>
      {listPageData.users.map((user) => {
        const user_component = user.user_detail({ onSelectId });
        return <div key={user.id}>{user_component}</div>;
      })}
    </>
  );
}

function TopLevelUserProfileWithDetails({ onGoBack, selectedId }) {
  // TODO replace this with the trick that causes graphql`...` literals to work
  const { queryReference } = useLazyReference(userDetailPageQuery, {
    id: selectedId,
  });
  const userProfileWithDetails = read(queryReference);
  return userProfileWithDetails({ onGoBack });
}

function FullPageLoading() {
  return <h1 className="mt-5">Loading...</h1>;
}
