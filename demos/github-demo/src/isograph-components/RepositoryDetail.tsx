import { iso } from '@iso';
import { RepoGitHubLink } from './RepoGitHubLink';
import { Route } from './GithubDemo';

export const IsStarred = iso(`
  field Starrable.IsStarred @component {
    stargazerCount
    viewerHasStarred
  }
`)(({ data }) => {
  return (
    <p>
      This item has been starred {data.stargazerCount} times,{' '}
      {data.viewerHasStarred ? 'including by the user' : 'but not by the user'}.
    </p>
  );
});

export const RepositoryDetail = iso(`
  field Query.RepositoryDetail($first: Int, $repositoryName: String, $repositoryOwner: String) @component {
    repository(name: $repositoryName, owner: $repositoryOwner) {
      IsStarred
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
`)(function RepositoryDetailComponent(
  { data },
  { setRoute }: { setRoute: (route: Route) => void },
) {
  const parent = data.repository?.parent;
  const repository = data.repository;
  if (repository == null) return null;
  return (
    <>
      <RepoGitHubLink filePath="demos/github-demo/src/isograph-components/RepositoryDetail.tsx">
        Repository Detail Component
      </RepoGitHubLink>
      <h1>{data.repository?.nameWithOwner}</h1>
      {parent != null ? (
        <h3>
          <small>Forked from</small>{' '}
          <parent.RepositoryLink setRoute={setRoute}>
            {parent.nameWithOwner}
          </parent.RepositoryLink>
        </h3>
      ) : null}
      <repository.IsStarred />
      <repository.pullRequests.PullRequestTable setRoute={setRoute} />
    </>
  );
});
