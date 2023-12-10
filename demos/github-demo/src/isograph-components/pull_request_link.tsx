import React from "react";
import { iso } from "@isograph/react";

import { ResolverParameterType as PullRequestProps } from "./__isograph/PullRequest/pull_request_link/reader.isograph";

import { Link } from "@mui/material";

export const pull_request_link = iso<
  PullRequestProps,
  ReturnType<typeof PullRequestLink>
>`
  PullRequest.pull_request_link @component {
    number,
    repository {
      name,
      owner {
        login,
      },
    },
  }
`(PullRequestLink);

function PullRequestLink(props: PullRequestProps) {
  return (
    <Link
      onClick={() =>
        props.setRoute({
          kind: "PullRequest",
          pullRequestNumber: props.data.number,
          repositoryName: props.data.repository.name,
          repositoryOwner: props.data.repository.owner.login,
        })
      }
      style={{ cursor: "pointer" }}
    >
      {props.children}
    </Link>
  );
}
