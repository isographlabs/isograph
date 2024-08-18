import Head from 'next/head';
import ThemeProvider from '@/src/theme';
import { Suspense } from 'react';
import { FullPageLoading } from '@/src/components/routes';
import { NewsfeedLoader } from '@/src/components/Newsfeed/NewsfeedRoute';

export default function Newsfeed() {
  return (
    <>
      <Head>
        <title>News feed</title>
      </Head>
      <ThemeProvider>
        <Suspense fallback={<FullPageLoading />}>
          <NewsfeedLoader />
        </Suspense>
      </ThemeProvider>
    </>
  );
}
