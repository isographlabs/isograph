import { iso } from '@isograph/react';
import type { ResolverParameterType as HomePageListParams } from '@iso/Query/HomePageList/reader.isograph';
import { RepoLink } from './RepoLink';
import { Button } from '@mui/material';

export const HomePageList = iso<HomePageListParams, ReturnType<typeof HomePageListComponent>>`
  field Query.HomePageList($first: Int!) @component {
    viewer {
      login,
      name,
      RepositoryList,
      __refetch,
    },
  }
`(HomePageListComponent);

function HomePageListComponent(props: HomePageListParams) {
  return (
    <>
      <RepoLink filePath="demos/github-demo/src/isograph-components/HomePage.tsx">
        Home Page List Component
      </RepoLink>
      <h1>rbalicki2's repository stats</h1>
      <Button onClick={() => props.data.viewer.__refetch()} variant="contained">
        Refetch viewer
      </Button>
      <props.data.viewer.RepositoryList setRoute={props.setRoute} />
    </>
  );
}
