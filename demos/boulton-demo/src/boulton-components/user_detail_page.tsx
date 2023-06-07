import * as React from "react";
import { bDeclare, read } from "@boulton/react";

// TODO @component seems to have no effect?
export const user_detail_page = bDeclare`
  Query.user_detail_page @component @fetchable {
    current_user {
      id,
      user_profile_with_details,
    },
  }
`;

export const user_profile_with_details = bDeclare`
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
