import { iso } from '@iso';
import type { ResolverParameterType as RepositoryDetailParams } from '@iso/Query/RepositoryDetail/reader';
import { RepoGitHubLink } from './RepoGitHubLink';

export const RepositoryDetail = iso<RepositoryDetailParams>`
  field Query.RepositoryDetail @component {
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
      <RepoGitHubLink filePath="demos/github-demo/src/isograph-components/RepositoryDetail.tsx">
        Repository Detail Component
      </RepoGitHubLink>
      <h1>{props.data.repository?.nameWithOwner}</h1>
      {parent != null ? (
        <h3>
          <small>Forked from</small>{' '}
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
