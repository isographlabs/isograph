import * as React from "react";
import { bDeclare } from "@boulton/react";

import { ResolverParameterType as UserDetailPageProps } from "./__boulton/Query__user_detail_page.boulton";
import { ResolverParameterType as UserProfileWithDetails } from "./__boulton/User__user_profile_with_details.boulton";

export const user_detail_page = bDeclare<
  UserDetailPageProps,
  ReturnType<typeof UserDetailPage>
>`
  Query.user_detail_page($id: ID!,) @fetchable @component {
    user(id: $id,) {
      id,
      user_profile_header,
      user_profile_with_details,
    },
  }
`(UserDetailPage);

// It's not very useful to make user_detail_page a component, since
// it basically calls user_profile_with_details /shrug
function UserDetailPage(props: UserDetailPageProps) {
  // This guy is a ref reader, but user_detail_page is not?
  return (
    <>
      {props.data.user.user_profile_header({})}
      <React.Suspense fallback="Loading...">
        {props.data.user.user_profile_with_details({
          onGoBack: props.onGoBack,
        })}
      </React.Suspense>
    </>
  );
}

export const user_profile_with_details = bDeclare<
  UserProfileWithDetails,
  unknown
>`
  User.user_profile_with_details @component {
    id,
    email,
    avatar_component,
    billing_details {
      id,
      billing_details_component,
    },
  }
`(function UserProfileWithDetails({ data, onGoBack }) {
  const [state, setState] = React.useState(true);
  return (
    <>
      <p>email: {data.email}</p>
      {data.avatar_component()}
      <p>
        <button onClick={onGoBack}>Go back</button>
      </p>
      <p>
        <button onClick={() => setState(!state)}>toggle</button>
        {state ? "true" : "false"}
      </p>
      {data.billing_details.billing_details_component({
        setStateToFalse: () => setState(false),
      })}
    </>
  );
});

export const user_profile_header = bDeclare`
  User.user_profile_header @component {
    id,
    name,
  }
`(({ data }) => {
  return <h1>Detail page for: {data.name}</h1>;
});
