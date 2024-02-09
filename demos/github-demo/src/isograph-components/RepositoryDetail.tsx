import { iso } from '@iso';
import { RepoGitHubLink } from './RepoGitHubLink';

export const RepositoryDetail = iso(`
  field Query.RepositoryDetail @component {
    repository(name: $repositoryName, owner: $repositoryOwner) {
      nameWithOwner
      parent {
        RepositoryLink
        nameWithOwner
      }

      pullRequests(last: $first) {
        PullRequestTable
      }
    }
  }
`)(function RepositoryDetailComponent(props) {
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
});
