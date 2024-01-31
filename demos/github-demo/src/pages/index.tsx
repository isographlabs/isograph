import Head from 'next/head';

import { GithubDemo } from '@/isograph-components/GithubDemo';

function makeNetworkRequest<T>(queryText: string, variables: any): Promise<T> {
  let promise = fetch('https://api.github.com/graphql', {
    method: 'POST',
    headers: {
      Authorization: 'Bearer ' + process.env.NEXT_PUBLIC_GITHUB_TOKEN,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ query: queryText, variables }),
  }).then((response) => response.json());
  return promise;
}

import { createTheme, ThemeProvider } from '@mui/material/styles';
import {
  IsographEnvironment,
  IsographEnvironmentProvider,
} from '@isograph/react';

const theme = createTheme({
  palette: {
    primary: {
      light: '#788caf',
      main: '#385276',
      dark: '#1a2f4a',
      contrastText: '#fff',
    },
    secondary: {
      light: '#ff7961',
      main: '#f28800',
      dark: '#e86600',
      contrastText: '#000',
    },
  },
});

const environment: IsographEnvironment = {
  store: {
    __ROOT: {},
  },
  missingFieldHandler: null,
  networkFunction: makeNetworkRequest,
};

export default function Home() {
  return (
    <>
      <Head>
        <title>Github Demo</title>
        <meta
          name="description"
          content="Demonstration of Isograph, used with Github's GraphQL API."
        />
      </Head>
      <ThemeProvider theme={theme}>
        <IsographEnvironmentProvider environment={environment}>
          <GithubDemo />
        </IsographEnvironmentProvider>
      </ThemeProvider>
    </>
  );
}
