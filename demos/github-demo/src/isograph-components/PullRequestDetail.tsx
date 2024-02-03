import React from 'react';

import { iso } from '@iso';
import { ResolverParameterType as PullRequestDetailsProps } from '@iso/Query/PullRequestDetail/reader';

import { Card, CardContent } from '@mui/material';
import { RepoGitHubLink } from './RepoGitHubLink';

export const PullRequestDetail = iso<PullRequestDetailsProps>`
  field Query.PullRequestDetail @component {
    repository(owner: $repositoryOwner, name: $repositoryName) {
      pullRequest(number: $pullRequestNumber) {
        title,
        bodyHTML,
        CommentList,
      },
    },
  }
`(PullRequestDetailComponent);

function PullRequestDetailComponent(props: PullRequestDetailsProps) {
  const repository = props.data.repository;
  if (repository === null) {
    return <h1>Repository not found</h1>;
  }

  const pullRequest = repository.pullRequest;
  if (pullRequest === null) {
    return <h1>Pull request not found</h1>;
  }

  return (
    <>
      <RepoGitHubLink filePath="demos/github-demo/src/isograph-components/PullRequestDetail.tsx">
        Pull Request Detail Component
      </RepoGitHubLink>

      <h1>{pullRequest.title}</h1>

      <Card variant="outlined">
        <CardContent>
          <div dangerouslySetInnerHTML={{ __html: pullRequest.bodyHTML }} />
        </CardContent>
      </Card>

      <h2>Comments</h2>
      <pullRequest.CommentList route={props.route} setRoute={props.setRoute} />
    </>
  );
}
