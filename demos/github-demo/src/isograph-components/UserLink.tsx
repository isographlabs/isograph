import { iso } from '@iso';

import { Link } from '@mui/material';

export const UserLink = iso(`
  field Actor.UserLink @component {
    login
  }
`)(function UserLinkComponent(props) {
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
});
