import Head from 'next/head';
import { GraphQLConfDemo } from '@/src/components/router';
import { createTheme, ThemeProvider } from '@mui/material/styles';
import { useRouter } from 'next/router';

export default function PetDetail() {
  const router = useRouter();

  // During SSR, id will be nullish. So, we just render the shell.
  // This isn't ideal, and we should figure out how to fix that!
  const id = router.query.id;

  return (
    <>
      <Head>
        <title>Robert&apos;s Pet List 3000</title>
      </Head>
      <ThemeProvider theme={theme}>
        {id && (
          <GraphQLConfDemo
            initialState={{
              kind: 'PetDetail',
              // @ts-expect-error
              id: router.query.id,
            }}
          />
        )}
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
