import * as React from "react";
import { bDeclare } from "@isograph/react";
import type { ResolverParameterType as FooQueryType } from "./__isograph/Query__foo_query.isograph";
import type { ResolverParameterType as UserDetailType } from "./__isograph/User__user_detail.isograph";

export const foo_query = bDeclare<FooQueryType, ReturnType<typeof FooQuery>>`
  Query.foo_query @fetchable {
    users {
      id,
      name,
    },
  }
`(FooQuery);
function FooQuery(param: FooQueryType) {
  return "stuff";
}

export const user_detail = bDeclare<
  UserDetailType,
  ReturnType<typeof UserListComponent>
>`
  User.user_detail @component {
    id,
    name,
    avatar_component,
  }
`(UserListComponent);

function UserListComponent({ data, onSelectId }: UserDetailType) {
  return (
    <>
      <h2>{data.name}</h2>
      {data.avatar_component({ foo: "bar" })}
      <button onClick={() => onSelectId(data.id)}>User details</button>
    </>
  );
}
