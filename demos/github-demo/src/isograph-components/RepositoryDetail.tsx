import { iso } from "@isograph/react";
import type { ResolverParameterType as RepositoryDetailParams } from "@iso/Query/RepositoryDetail/reader.isograph";
import { RepoLink } from "./RepoLink";

export const RepositoryDetail = iso<
  RepositoryDetailParams,
  ReturnType<typeof RepositoryDetailComponent>
>`
  Query.RepositoryDetail @component {
    repository(name: $repositoryName, owner: $repositoryOwner) {
      nameWithOwner,
      parent {
        RepositoryLink,
        nameWithOwner,
      },

      pullRequests(last: $first) {
        PullRequestTable,
      },
    },
  }
`(RepositoryDetailComponent);

function RepositoryDetailComponent(props: RepositoryDetailParams) {
  const parent = props.data.repository?.parent;
  const repository = props.data.repository;
  if (repository == null) return null;
  return (
    <>
      <RepoLink filePath="demos/github-demo/src/isograph-components/repository_detail.tsx">
        Repository Detail Component
      </RepoLink>
      <h1>{props.data.repository?.nameWithOwner}</h1>
      {parent != null ? (
        <h3>
          <small>Forked from</small>{" "}
          <parent.RepositoryLink
            setRoute={props.setRoute}
            children={parent.nameWithOwner}
          />
        </h3>
      ) : null}
      <repository.pullRequests.PullRequestTable setRoute={props.setRoute} />
      {/* <div>Stargazer count: {props.data.repository?.stargazerCount}</div> */}
    </>
  );
}
