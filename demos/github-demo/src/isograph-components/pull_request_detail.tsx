import React from "react";

import { iso } from "@isograph/react";
import { ResolverParameterType as PullRequestDetailsProps } from "./__isograph/Query/pull_request_detail/reader.isograph";

import { Card, CardContent } from "@mui/material";
import { RepoLink } from "./RepoLink";

export const pull_request_detail = iso<
  PullRequestDetailsProps,
  ReturnType<typeof PullRequestDetail>
>`
  Query.pull_request_detail @component {
    repository(owner: $repositoryOwner, name: $repositoryName) {
      pullRequest(number: $pullRequestNumber) {
        title,
        bodyHTML,
        comment_list,
      },
    },
  }
`(PullRequestDetail);

function PullRequestDetail(props: PullRequestDetailsProps) {
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
      <RepoLink filePath="demos/github-demo/src/isograph-components/pull_request_detail.tsx">
        Pull Request Detail Component
      </RepoLink>

      <h1>{pullRequest.title}</h1>

      <Card variant="outlined">
        <CardContent>
          <div dangerouslySetInnerHTML={{ __html: pullRequest.bodyHTML }} />
        </CardContent>
      </Card>

      <h2>Comments</h2>
      {pullRequest.comment_list({
        route: props.route,
        setRoute: props.setRoute,
      })}
    </>
  );
}
