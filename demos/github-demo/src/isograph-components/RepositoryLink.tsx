import { iso } from '@iso';

import { Link } from '@mui/material';

export const RepositoryLink = iso(`
  field Repository.RepositoryLink @component {
    id,
    name,
    owner {
      login,
    },
  }
`)(function RepositoryLinkComponent(props) {
  return (
    <Link
      onClick={() =>
        props.setRoute({
          kind: 'Repository',
          repositoryName: props.data.name,
          repositoryOwner: props.data.owner.login,
          repositoryId: props.data.id,
        })
      }
      style={{ cursor: 'pointer' }}
    >
      {props.children}
    </Link>
  );
});
