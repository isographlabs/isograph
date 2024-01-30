import { iso } from '@isograph/react';
import type { ResolverParameterType as UserDetailParams } from '@iso/Query/UserDetail/reader.isograph';
import { RepoLink } from './RepoLink';

export const UserDetail = iso<UserDetailParams>`
  field Query.UserDetail @component {
    user(login: $userLogin) {
      name,
      RepositoryList,
    },
  }
`(UserDetailComponent);

function UserDetailComponent(props: UserDetailParams) {
  console.log('user detail props.data:', props.data);
  const user = props.data.user;
  if (user == null) {
    return <h1>user not found</h1>;
  }

  return (
    <>
      <RepoLink filePath="demos/github-demo/src/isograph-components/UserDetail.tsx">
        User Detail Component
      </RepoLink>
      <h1>{user.name}</h1>
      <user.RepositoryList setRoute={props.setRoute} />
    </>
  );
}
