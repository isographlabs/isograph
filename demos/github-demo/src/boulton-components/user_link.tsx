import { bDeclare } from "@isograph/react";

import type { ResolverParameterType as UserLinkParams } from "./__isograph/Actor__user_link.isograph";

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
