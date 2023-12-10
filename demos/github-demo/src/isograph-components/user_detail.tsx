import { iso } from "@isograph/react";
import type { ResolverParameterType as UserDetailParams } from "./__isograph/Query/user_detail/reader.isograph";
import { RepoLink } from "./RepoLink";

export const user_detail = iso<UserDetailParams, ReturnType<typeof UserDetail>>`
  Query.user_detail @component {
    user(login: $userLogin) {
      name,
      repository_list,
    },
  }
`(UserDetail);

function UserDetail(props: UserDetailParams) {
  console.log("user detail props.data:", props.data);
  const user = props.data.user;
  if (user == null) {
    return <h1>user not found</h1>;
  }

  return (
    <>
      <RepoLink filePath="demos/github-demo/src/isograph-components/user_detail.tsx">
        User Detail Component
      </RepoLink>
      <h1>{user.name}</h1>
      {user.repository_list({ setRoute: props.setRoute })}
    </>
  );
}
