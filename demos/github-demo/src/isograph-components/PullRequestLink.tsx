import React, { ReactNode } from 'react';
import { iso } from '@iso';

import { Link } from '@mui/material';
import { Route } from './GithubDemo';
import { useClientSideDefer } from '@isograph/react';

export const PullRequestLink = iso(`
  field PullRequest.PullRequestLink @component {
    number
    repository {
      name
      owner {
        login
        asUser {
          id
          bio
        }
      }
    }
  }
`)(function PullRequestLinkComponent(
  { data },
  {
    setRoute,
    children,
  }: {
    setRoute: (route: Route) => void;
    children: ReactNode;
  },
) {
  const user = useClientSideDefer(data.repository.owner.asUser);
  return (
    <Link
      onClick={() =>
        setRoute({
          kind: 'PullRequest',
          pullRequestNumber: data.number,
          repositoryName: data.repository.name,
          repositoryOwner: data.repository.owner.login,
        })
      }
      style={{ cursor: 'pointer' }}
    >
      {children}
      {JSON.stringify(user)}
    </Link>
  );
});
