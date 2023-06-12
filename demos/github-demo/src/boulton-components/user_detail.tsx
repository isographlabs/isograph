import { bDeclare } from "@boulton/react";
import type { ResolverParameterType as UserDetailParams } from "./__boulton/Query__user_detail.boulton";

export const user_detail = bDeclare<
  UserDetailParams,
  ReturnType<typeof UserDetail>
>`
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
