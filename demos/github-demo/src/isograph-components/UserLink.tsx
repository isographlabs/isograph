import { iso } from '@iso';

import { Link } from '@mui/material';
import { ReactNode } from 'react';
import { Route } from './GithubDemo';

export const UserLink = iso(`
  field Actor.UserLink @component {
    login
  }
`)(function UserLinkComponent(
  { data },
  {
    setRoute,
    children,
  }: {
    setRoute: (route: Route) => void;
    children: ReactNode;
  },
) {
  return (
    <Link
      onClick={() =>
        setRoute({
          kind: 'User',
          userLogin: data.login,
        })
      }
      style={{ cursor: 'pointer' }}
    >
      {children}
    </Link>
  );
});
