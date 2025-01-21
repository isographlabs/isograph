import { iso } from '@iso';
import { Link } from '@mui/material';
import { ReactNode } from 'react';
import { Route } from './GithubDemo';

export const RepositoryLink = iso(`
  field Repository.RepositoryLink @component {
    id
    name
    owner {
      login
    }
  }
`)(function RepositoryLinkComponent(
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
          kind: 'Repository',
          repositoryName: data.name,
          repositoryOwner: data.owner.login,
          repositoryId: data.id,
        })
      }
      style={{ cursor: 'pointer' }}
    >
      {children}
    </Link>
  );
});
