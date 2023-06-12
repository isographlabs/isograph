import { bDeclare } from "@boulton/react";
import type { ResolverParameterType as UserRepositoryListParams } from "./__boulton/User__repository_list.boulton";

import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
} from "@mui/material";

export const repository_list = bDeclare<
  UserRepositoryListParams,
  ReturnType<typeof UserRepositoryList>
>`
  User.repository_list @component {
    repositories(last: $first,) {
      edges {
        node {
          id,
          repository_link,
          name,
          nameWithOwner,
          description,
          forkCount,
          pullRequests(first: $first,) {
            totalCount,
          },
          stargazerCount,
          watchers(first: $first,) {
            totalCount,
          },
        },
      },
    },
  }
`(UserRepositoryList);

function UserRepositoryList(props: UserRepositoryListParams) {
  const repositories = [...props.data.repositories.edges].reverse();
  return (
    <Table>
      <TableHead>
        <TableRow>
          <TableCell>Repository</TableCell>
          <TableCell>Stars</TableCell>
          <TableCell>Forks</TableCell>
          <TableCell>Total PRs</TableCell>
          <TableCell>Watchers</TableCell>
        </TableRow>
      </TableHead>
      <TableBody>
        {repositories.map((data) => {
          if (data == null || data.node == null) {
            return null;
          }
          const { node } = data;
          return (
            <TableRow key={node.id}>
              <TableCell>
                {node.repository_link({
                  setRoute: props.setRoute,
                  children: node.nameWithOwner,
                })}
              </TableCell>
              <TableCell>{node.stargazerCount}</TableCell>
              <TableCell>{node.forkCount}</TableCell>
              <TableCell>{node.pullRequests?.totalCount}</TableCell>
              <TableCell>{node.watchers?.totalCount}</TableCell>
            </TableRow>
          );
        })}
      </TableBody>
    </Table>
  );
}
