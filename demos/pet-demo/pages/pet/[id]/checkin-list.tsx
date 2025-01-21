import { PetCheckinListLoader } from '@/src/components/PetCheckinListRoute';
import { FullPageLoading } from '@/src/components/routes';
import ThemeProvider from '@/src/theme';
import Head from 'next/head';
import { useRouter } from 'next/router';
import { Suspense } from 'react';

export default function PetDetail() {
  const router = useRouter();

  // During SSR, id will be nullish. So, we just render the shell.
  // This isn't ideal, and we should figure out how to fix that!
  const id = router.query.id;
  if (id == null || Array.isArray(id)) {
    return;
  }

  return (
    <>
      <Head>
        <title>Robert&apos;s Pet List 3000</title>
      </Head>
      <ThemeProvider>
        <Suspense fallback={<FullPageLoading />}>
          <PetCheckinListLoader route={{ kind: 'PetCheckinList', id }} />
        </Suspense>
      </ThemeProvider>
    </>
  );
}
