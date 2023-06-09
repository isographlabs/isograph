import * as React from "react";
import { bDeclare } from "@boulton/react";
import type { ResolverParameterType as QueryUserListPage } from "./__boulton/Query__user_list_page.boulton";
import type { ResolverParameterType as FooQueryType } from "./__boulton/Query__foo_query.boulton";
import type { ResolverParameterType as UserListComponentType } from "./__boulton/User__user_list_component.boulton";

export const user_list_page = bDeclare<QueryUserListPage>`
  Query.user_list_page(
    $bar: String!,
  ) @fetchable {
    byah(foo: $bar,),
    users {
      id,
      user_list_component,
    },
  }
`;

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

export const user_list_component = bDeclare<
  UserListComponentType,
  ReturnType<typeof UserListComponent>
>`
  User.user_list_component @component {
    id,
    name,
    avatar_component,
  }
`(UserListComponent);

function UserListComponent({ data, onSelectId }: UserListComponentType) {
  return (
    <>
      <h2>{data.name}</h2>
      {data.avatar_component({ foo: "bar" })}
      <button onClick={() => onSelectId(data.id)}>User details</button>
    </>
  );
}
