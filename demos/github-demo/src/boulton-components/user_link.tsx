import { bDeclare } from "@boulton/react";

import type { ResolverParameterType as UserLinkParams } from "./__boulton/Actor__user_link.boulton";

import { Link } from "@mui/material";

export const user_link = bDeclare<UserLinkParams, ReturnType<typeof UserLink>>`
  Actor.user_link @component {
    login,
  }
`(UserLink);

function UserLink(props: UserLinkParams) {
  return (
    <Link
      onClick={() =>
        props.setRoute({
          kind: "User",
          userLogin: props.data.login,
        })
      }
      style={{ cursor: "pointer" }}
    >
      {props.children}
    </Link>
  );
}
