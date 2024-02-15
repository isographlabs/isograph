import Layout from '@theme/Layout';

import { TwitterTweetEmbed } from 'react-twitter-embed';

export default function Home(): JSX.Element {
  return (
    <Layout
      title={`Isograph, the UI framework for teams that move fast without breaking things`}
      description="Isograph, the UI framework for teams that move fast without breaking things"
    >
      <main>
        <div
          style={{
            flexDirection: 'row',
            display: 'flex',
            justifyContent: 'center',
          }}
        >
          <div style={{ minWidth: 500 }}>
            <TwitterTweetEmbed tweetId={'1758185952759992507'} />
          </div>
        </div>
      </main>
    </Layout>
  );
}
