import React from "react";
import { bDeclare } from "@isograph/react";

import { ResolverParameterType as PullRequestProps } from "./__isograph/PullRequest__pull_request_link.isograph";

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
