import { NewsfeedLoader } from '@/src/components/Newsfeed/NewsfeedRoute';
import { FullPageLoading } from '@/src/components/routes';
import ThemeProvider from '@/src/theme';
import Head from 'next/head';
import { Suspense } from 'react';

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
