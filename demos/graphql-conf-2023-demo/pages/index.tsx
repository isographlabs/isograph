import Head from 'next/head';
import {
  DataId,
  Link,
  StoreRecord,
  setMissingFieldHandler,
  defaultMissingFieldHandler,
  setNetwork,
  clearStore,
} from '@isograph/react';
import { GraphQLConfDemo } from '@/src/components/router';
import { createTheme, ThemeProvider } from '@mui/material/styles';

function makeNetworkRequest<T>(queryText: string, variables: any): Promise<T> {
  let promise = fetch('http://localhost:4000/graphql', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ query: queryText, variables }),
  }).then((response) => response.json());
  return promise;
}
setNetwork(makeNetworkRequest);
setMissingFieldHandler(
  (
    storeRecord: StoreRecord,
    root: DataId,
    fieldName: string,
    arguments_: { [index: string]: any } | null,
    variables: { [index: string]: any } | null,
  ): Link | undefined => {
    if (typeof window !== 'undefined' && window.__LOG) {
      console.log('Missing field handler called', {
        storeRecord,
        root,
        fieldName,
        arguments_,
        variables,
      });
    }
    const val = defaultMissingFieldHandler(
      storeRecord,
      root,
      fieldName,
      arguments_,
      variables,
    );
    if (val == undefined) {
      // This is the custom missing field handler
      //
      // N.B. this **not** correct. We need to pass the correct variables/args here.
      // But it works for this demo.
      if (fieldName === 'pet' && variables?.id != null && root === '__ROOT') {
        return { __link: variables.id };
      }
    } else {
      return val;
    }
  },
);

export default function Home() {
  return (
    <>
      <Head>
        <title>Robert's Pet List 3000</title>
        <meta name="description" content="Demo for GraphQL Conf 2023" />
      </Head>
      <ThemeProvider theme={theme}>
        <GraphQLConfDemo />
      </ThemeProvider>
    </>
  );
}

const theme = createTheme({
  spacing: 4,
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

export async function getServerSideProps() {
  clearStore();
  return { props: {} };
}
