import { iso } from "@isograph/react";

import type { ResolverParameterType as RepositoryLinkParams } from "./__isograph/Repository/repository_link/reader.isograph";

import { Link } from "@mui/material";

export const repository_link = iso<
  RepositoryLinkParams,
  ReturnType<typeof RepositoryLink>
>`
  Repository.repository_link @component {
    id,
    name,
    owner {
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
