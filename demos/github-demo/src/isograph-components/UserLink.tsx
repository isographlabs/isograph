import { iso } from '@iso';
import XIcon from '@mui/icons-material/X';
import { Link } from '@mui/material';
import { ReactNode } from 'react';
import { Route } from './GithubDemo';

export const UserLink = iso(`
  field Actor.UserLink @component {
    login
    asUser {
      id
      twitterUsername
    }
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
  if (!data.asUser) {
    return data.login;
  }
  return (
    <>
      <Link
        onClick={() =>
          data.asUser &&
          setRoute({
            kind: 'User',
            userLogin: data.login,
          })
        }
        style={{ cursor: 'pointer' }}
      >
        {children}
      </Link>
      &nbsp;
      {data.asUser?.twitterUsername && (
        <Link
          href={`https://x.com/${data.asUser.twitterUsername}`}
          target="_blank"
        >
          {data.asUser.twitterUsername && <XIcon />}
        </Link>
      )}
    </>
  );
});
