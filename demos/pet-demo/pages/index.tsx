import Head from 'next/head';
import ThemeProvider from '@/src/theme';
import { HomeRouteLoader } from '@/src/components/HomeRoute';
import { Suspense } from 'react';
import { FullPageLoading } from '@/src/components/routes';

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
