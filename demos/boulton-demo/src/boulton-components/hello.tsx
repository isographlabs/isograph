import { bDeclare } from "@boulton/react";

export const hello_component = bDeclare`
  Query.hello_component {
    hello,
    current_user {
      id,
      avatar_component,
    },
  }
`((data) => {
  return "Hello, " + data.hello;
});
