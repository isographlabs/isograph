import React from "react";
import { bDeclare } from "@boulton/react";

import { ResolverParameterType as PullRequestProps } from "./__boulton/PullRequest__pull_request_link.boulton";

import { Link } from "@mui/material";

export const pull_request_link = bDeclare<
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
