import { iso } from "@isograph/react";

import type { ResolverParameterType as UserLinkParams } from "@iso/Actor/user_link/reader.isograph";

import { Link } from "@mui/material";

export const user_link = iso<UserLinkParams, ReturnType<typeof UserLink>>`
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
