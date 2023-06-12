import { bDeclare } from "@boulton/react";

import type { ResolverParameterType as RepositoryLinkParams } from "./__boulton/Repository__repository_link.boulton";

import { Link } from "@mui/material";

export const repository_link = bDeclare<
  RepositoryLinkParams,
  ReturnType<typeof RepositoryLink>
>`
  Repository.repository_link @component {
    id,
    name,
    owner {
      id, 
      login,
    },
  }
`(RepositoryLink);

function RepositoryLink(props: RepositoryLinkParams) {
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
