import Layout from '@theme/Layout';
import Components from '../components/Components';
import Fetching from '../components/Fetching';
import HomepageHeader from '../components/Header';
import IntroducingIsograph from '../components/IntroducingIsograph';
import IsIsographRightForMe from '../components/IsIsographRightForMe';
import IsographFeatures from '../components/IsographFeatures';
import ProblemStatement from '../components/ProblemStatement';
import YoutubeEmbed from '../components/YoutubeEmbed';

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
