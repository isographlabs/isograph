import { iso } from "@isograph/react";
import type { ResolverParameterType as UserDetailParams } from "./__isograph/Query__user_detail.isograph";

export const user_detail = iso<UserDetailParams, ReturnType<typeof UserDetail>>`
  Query.user_detail @component {
    user(login: $userLogin,) {
      id,
      name,
      repository_list,
    },
  }
`(UserDetail);

function UserDetail(props: UserDetailParams) {
  console.log("repo detail", props.data);
  const user = props.data.user;
  if (user == null) {
    return <h1>user not found</h1>;
  }

  return (
    <>
      <h1>{user.name}</h1>
      {user.repository_list({ setRoute: props.setRoute })}
    </>
  );
}
