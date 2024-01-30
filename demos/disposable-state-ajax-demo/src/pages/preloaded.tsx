import Head from 'next/head';

import { PreloadedPostsWrapper } from '@/components/PreloadedPostsPage';

export default function Home() {
  return (
    <>
      <Head>
        <title>Preloading demonstration</title>
        <meta
          name="description"
          content="Demonstration of network requests preloaded (i.e. imperatively) using react-disposable-state"
        />
      </Head>
      <div className="container">
        <PreloadedPostsWrapper />
      </div>
    </>
  );
}
