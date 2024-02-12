import Layout from '@theme/Layout';

import YoutubeEmbed from '../components/YoutubeEmbed';
import ProblemStatement from '../components/ProblemStatement';
import IsographFeatures from '../components/IsographFeatures';
import HomepageHeader from '../components/Header';
import Components from '../components/Components';
import Fetching from '../components/Fetching';
import IsIsographRightForMe from '../components/IsIsographRightForMe';
import IntroducingIsograph from '../components/IntroducingIsograph';

export default function Home(): JSX.Element {
  return (
    <Layout
      title={`Isograph, the UI framework for teams that move fast without breaking things`}
      description="Isograph, the UI framework for teams that move fast without breaking things"
    >
      <HomepageHeader />
      <main>
        <ProblemStatement />
        <IntroducingIsograph />
        <IsographFeatures />
        <YoutubeEmbed />
        <Components />
        <Fetching />
        <IsIsographRightForMe />
      </main>
    </Layout>
  );
}
