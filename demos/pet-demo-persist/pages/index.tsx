import { HomeRouteLoader } from '@/src/components/HomeRoute';
import { FullPageLoading } from '@/src/components/routes';
import ThemeProvider from '@/src/theme';
import Head from 'next/head';
import { Suspense } from 'react';

export default function Home() {
  return (
    <>
      <Head>
        <title>Robert&apos;s Pet List 3000</title>
      </Head>
      <ThemeProvider>
        <Suspense fallback={<FullPageLoading />}>
          <HomeRouteLoader />
        </Suspense>
      </ThemeProvider>
    </>
  );
}
