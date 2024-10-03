import { iso } from '@iso';
import { RepoGitHubLink } from './RepoGitHubLink';
import { Route } from './GithubDemo';

export const UserDetail = iso(`
  field Query.UserDetail($userLogin: String) @component {
    user(login: $userLogin) {
      name
      RepositoryList
    }
  }
`)(function UserDetailComponent(
  { data },
  {
    setRoute,
  }: {
    setRoute: (route: Route) => void;
  },
) {
  const user = data.user;
  if (user == null) {
    return <h1>user not found</h1>;
  }

  return (
    <>
      <RepoGitHubLink filePath="demos/github-demo/src/isograph-components/UserDetail.tsx">
        User Detail Component
      </RepoGitHubLink>
      <h1>{user.name}</h1>
      <user.RepositoryList setRoute={setRoute} />
    </>
  );
});
