import { iso } from '@isograph/react';

import type { ResolverParameterType as UserLinkParams } from '@iso/Actor/UserLink/reader';

import { Link } from '@mui/material';

export const UserLink = iso<UserLinkParams>`
  field Actor.UserLink @component {
    login,
  }
`(UserLinkComponent);

function UserLinkComponent(props: UserLinkParams) {
  return (
    <Link
      onClick={() =>
        props.setRoute({
          kind: 'User',
          userLogin: props.data.login,
        })
      }
      style={{ cursor: 'pointer' }}
    >
      {props.children}
    </Link>
  );
}
