import * as React from "react";
import { bDeclare, read } from "@boulton/react";

export const user_page_with_details = bDeclare`
  Query.user_detail_page {
    current_user {
      id,
      user_profile_with_details,
    },
  }
`;

export const user_profile_with_details = bDeclare`
  User.user_profile_with_details {
    id,
    name,
    email,
    avatar_component,
    billing_details {
      id,
      billing_details_component,
    },
  }
`((data) => {
  const avatar = read(data.avatar_component);
  return (onGoBack: () => void) => (
    <>
      <h1>User detail page for {data.name}</h1>
      <p>email: {data.email}</p>
      {avatar}
      <p>
        <a onClick={onGoBack}>Go back</a>
      </p>
      <React.Suspense fallback={<p>Loading...</p>}>
        <QueryRefReader
          queryRef={data.billing_details.billing_details_component}
        />
      </React.Suspense>
    </>
  );
});

// This is a hack! This can and will be done by the compiler.
function QueryRefReader({ queryRef }) {
  const data = read(queryRef);
  return data;
}
