import React from "react";
import { iso } from "@isograph/react";

import { ResolverParameterType as PullRequestProps } from "@iso/PullRequest/pull_request_link/reader.isograph";

import { Link } from "@mui/material";

export const PullRequestLink = iso<
  PullRequestProps,
  ReturnType<typeof PullRequestLinkComponent>
>`
  PullRequest.PullRequestLink @component {
    number,
    repository {
      name,
      owner {
        login,
      },
    },
  }
`(PullRequestLinkComponent);

function PullRequestLinkComponent(props: PullRequestProps) {
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
