import Head from 'next/head';
import { createTheme, ThemeProvider } from '@mui/material/styles';

import { GithubDemo } from '@/isograph-components/GithubDemo';

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
        <GithubDemo />
      </ThemeProvider>
    </>
  );
}
