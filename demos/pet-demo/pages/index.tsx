import Head from 'next/head';
import { GraphQLConfDemo } from '@/src/components/router';
import { createTheme, ThemeProvider } from '@mui/material/styles';

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
