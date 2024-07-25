import Head from 'next/head';
import ThemeProvider from '@/src/theme';
import { useRouter } from 'next/router';
import { FullPageLoading } from '@/src/components/routes';
import { Suspense } from 'react';
import { PetByNameRouteLoader } from '@/src/components/PetByName';

export default function PetDetail() {
  const router = useRouter();

  // During SSR, id will be nullish. So, we just render the shell.
  // This isn't ideal, and we should figure out how to fix that!
  const name = router.query.name;
  if (name == null || Array.isArray(name)) {
    return;
  }

  return (
    <>
      <Head>
        <title>Robert&apos;s Pet List 3000</title>
      </Head>
      <ThemeProvider>
        <Suspense fallback={<FullPageLoading />}>
          <PetByNameRouteLoader
            route={{
              kind: 'PetByName',
              name,
            }}
          />
        </Suspense>
      </ThemeProvider>
    </>
  );
}
