import * as React from "react";
import { bDeclare, read } from "@boulton/react";

export const user_list_page = bDeclare`
  Query.user_list_page {
    users {
      id,
      user_list_component,
    },
  }
`;

export const user_list_component = bDeclare`
  User.user_list_component {
    id,
    name,
    avatar_component,
  }
`((data) => {
  const avatar = read(data.avatar_component);
  return (selectUser: (id: string) => void /* these are runtime props */) => (
    <>
      <h1>{data.name}</h1>
      {avatar}
      <button onClick={() => selectUser(data.id)}>User details</button>
    </>
  );
});
