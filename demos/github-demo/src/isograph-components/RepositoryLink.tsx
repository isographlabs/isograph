import { iso } from "@isograph/react";

import type { ResolverParameterType as RepositoryLinkParams } from "@iso/Repository/RepositoryLink/reader.isograph";

import { Link } from "@mui/material";

export const RepositoryLink = iso<
  RepositoryLinkParams,
  ReturnType<typeof RepositoryLinkComponent>
>`
  field Repository.RepositoryLink @component {
    id,
    name,
    owner {
      login,
    },
  }
`(RepositoryLinkComponent);

function RepositoryLinkComponent(props: RepositoryLinkParams) {
  return (
    <Link
      onClick={() =>
        props.setRoute({
          kind: "Repository",
          repositoryName: props.data.name,
          repositoryOwner: props.data.owner.login,
          repositoryId: props.data.id,
        })
      }
      style={{ cursor: "pointer" }}
    >
      {props.children}
    </Link>
  );
}
