import Head from 'next/head';
import { GraphQLConfDemo } from '@/src/components/router';
import ThemeProvider from '@/src/theme';
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
      <ThemeProvider>
        {id && (
          <GraphQLConfDemo
            initialState={{
              kind: 'PetDetailDeferred',
              // @ts-expect-error
              id: router.query.id,
            }}
          />
        )}
      </ThemeProvider>
    </>
  );
}
