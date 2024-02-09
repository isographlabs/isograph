import React from 'react';
import { iso } from '@iso';

import { Link } from '@mui/material';

export const PullRequestLink = iso(`
  field PullRequest.PullRequestLink @component {
    number
    repository {
      name
      owner {
        login
      }
    }
  }
`)(function PullRequestLinkComponent(props) {
  return (
    <Link
      onClick={() =>
        props.setRoute({
          kind: 'PullRequest',
          pullRequestNumber: props.data.number,
          repositoryName: props.data.repository.name,
          repositoryOwner: props.data.repository.owner.login,
        })
      }
      style={{ cursor: 'pointer' }}
    >
      {props.children}
    </Link>
  );
});
