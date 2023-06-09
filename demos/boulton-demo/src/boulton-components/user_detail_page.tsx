import * as React from "react";
import { bDeclare } from "@boulton/react";

import { ResolverParameterType as UserDetailPageProps } from "./__boulton/Query__user_detail_page.boulton";
import { ResolverParameterType as UserProfileWithDetails } from "./__boulton/User__user_profile_with_details.boulton";

// TODO @component seems to have no effect?
export const user_detail_page = bDeclare<
  UserDetailPageProps,
  ReturnType<typeof UserDetailPage>
>`
  Query.user_detail_page @fetchable @component {
    current_user {
      id,
      user_profile_with_details,
    },
  }
`(UserDetailPage);
function UserDetailPage(props: UserDetailPageProps) {
  console.log("user detail props", props);
  return props.data.current_user.user_profile_with_details({
    onGoBack: props.onGoBack,
  });
}

export const user_profile_with_details = bDeclare<
  UserProfileWithDetails,
  unknown
>`
  User.user_profile_with_details @component {
    id,
    name,
    email,
    avatar_component,
    billing_details {
      id,
      billing_details_component,
    },
  }
`(function UserProfileWithDetails({ data, onGoBack }) {
  console.log("user profile with details", data, onGoBack);
  const [state, setState] = React.useState(true);
  return (
    <>
      <h1>User detail page for {data.name}</h1>
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
        additionalRuntimePropsGoHere: "unused",
        setStateToFalse: () => setState(false),
      })}
    </>
  );
});
