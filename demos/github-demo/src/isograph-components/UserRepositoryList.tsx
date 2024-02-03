import { iso } from '@iso';
import type { ResolverParameterType as UserRepositoryListParams } from '@iso/User/RepositoryList/reader';

import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
} from '@mui/material';

export const RepositoryList = iso<UserRepositoryListParams>`
  field User.RepositoryList @component {
    repositories(last: 10) {
      edges {
        node {
          id,
          RepositoryLink,
          name,
          nameWithOwner,
          description,
          forkCount,
          pullRequests(first: $first) {
            totalCount,
          },
          stargazerCount,
          watchers(first: $first) {
            totalCount,
          },
        },
      },
    },
  }
`(UserRepositoryListComponent);

function UserRepositoryListComponent(props: UserRepositoryListParams) {
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
                <node.RepositoryLink
                  setRoute={props.setRoute}
                  children={node.nameWithOwner}
                />
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
