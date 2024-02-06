import { iso } from '@iso';
import { RepoGitHubLink } from './RepoGitHubLink';

export const UserDetail = iso(`
  field Query.UserDetail @component {
    user(login: $userLogin) {
      name,
      RepositoryList,
    },
  }
`)(function UserDetailComponent(props) {
  console.log('user detail props.data:', props.data);
  const user = props.data.user;
  if (user == null) {
    return <h1>user not found</h1>;
  }

  return (
    <>
      <RepoGitHubLink filePath="demos/github-demo/src/isograph-components/UserDetail.tsx">
        User Detail Component
      </RepoGitHubLink>
      <h1>{user.name}</h1>
      <user.RepositoryList setRoute={props.setRoute} />
    </>
  );
});
