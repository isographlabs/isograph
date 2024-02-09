import { iso } from '@iso';
import { RepoGitHubLink } from './RepoGitHubLink';
import { Button } from '@mui/material';

export const HomePageList = iso(`
  field Query.HomePageList($first: Int!) @component {
    viewer {
      login
      name
      RepositoryList
      __refetch
    }
  }
`)(function HomePageListComponent(props) {
  return (
    <>
      <RepoGitHubLink filePath="demos/github-demo/src/isograph-components/HomePage.tsx">
        Home Page List Component
      </RepoGitHubLink>
      <h1>rbalicki2's repository stats</h1>
      <Button onClick={() => props.data.viewer.__refetch()} variant="contained">
        Refetch viewer
      </Button>
      <props.data.viewer.RepositoryList setRoute={props.setRoute} />
    </>
  );
});
