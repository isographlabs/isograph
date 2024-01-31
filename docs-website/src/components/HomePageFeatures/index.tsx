import Heading from '@theme/Heading';
import styles from './styles.module.css';
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CodeBlock from '@theme/CodeBlock';

const FeatureList = [
  {
    title: 'No boilerplate',
    description: (
      <>
        <div>
          Isograph eliminates the boilerplate of working with GraphQL. No more
          fussing with props and fragment references — Isograph does all of that
          for you.
        </div>
        <br />
        <div>
          All you need to do is to define a component and the fields it needs.
          Isograph takes care of the rest.
        </div>
      </>
    ),
  },
  {
    title: 'Unparalleled performance',
    description: (
      <>
        <div>
          Isograph gives you great performance out of the box &mdash; like a
          single query per view that fetches exactly the right fields.
        </div>
      </>
    ),
  },
  {
    title: 'Tremendous stability',
    description: (
      <>
        <div>
          Local reasoning is enough to ensure application stability, because the
          data one component requests cannot affect the data another component
          receives. So engineers can feel safe moving fast.
        </div>
        <br />
        <div>
          But Isograph goes further: with mutations, Isograph ensures that all
          the fields you need are refetched, without requiring any work by the
          developer.
        </div>
      </>
    ),
  },
];

function Feature({ title, description }) {
  return (
    <div className="col col--4">
      <div className="text--center padding-horiz--md">
        <Heading as="h3">{title}</Heading>
        <div style={{ paddingBottom: 20 }}>{description}</div>
      </div>
    </div>
  );
}

const CodeBlocks = {
  home_page: `
import {
  read,
  useLazyReference,
  isoFetch,
} from "@isograph/react";
import Entrypoint from "@iso/Query/HomePage/entrypoint.isograph";

export function HomePageRoute() {
  // Step 1: Make a network request (during render) for the
  // Query.HomePage client-defined field.
  const { queryReference } = useLazyReference<typeof Entrypoint>(
    // Note that calling isoFetch here **generates** a query
    // at compile time!
    isoFetch${'`'}Query.HomePage${'`'},
    { /* Query variables */ }
  );
  
  // Step 2: Attempt to read the query reference. This will
  // return the value of the Query.HomePage field
  // when the network request is complete.
  const HomePage = read(queryReference);

  // Step 3: Render the resulting component.
  const additionalProps = {};
  return <HomePage {...additionalProps} />;
}
  `,
  home_page_component: `
import { iso } from "@isograph/react";
import {
  ResolverParameterType as HomePageComponentParams,
} from "@iso/Query/HomePage/reader.isograph";

// Step 1: Export the home_page_component and call iso
export const HomePage = iso<
  // Step 2: Pass type parameters to iso. (This will not be
  // necessary soon.)
  HomePageComponentParams,
>${'`'}
  # Step 3: Define a field named home_page_component on the
  # Query type, and tell the Isograph compiler that it is a
  # React @component
  Query.HomePage @component {
    # Step 4: Select whatever fields you'll need, including
    # other client-defined fields like Avatar.
    viewer {
      first_name,
      last_name,
      Avatar,
    },
  }
${'`'}(HomePageComponent);

function HomePageComponent({ data }: HomePageComponentParams) {
  // Step 5: Render your component
  const viewer = data.viewer;
  return <>
    <h1>Hello {viewer?.first_name} {viewer?.last_name}!</h1>
    <data.Avatar />
  </>
}
  `,
  avatar_component: `
import { iso } from "@isograph/react";
import {
  ResolverParameterType as AvatarProps,
} from "@iso/User/Avatar/reader.isograph";
import MyComponentLibraryAvatar from 'my-component-library';
  
export const Avatar = iso<AvatarProps>${'`'}
  User.Avatar @component {
    avatarUrl,
  }
${'`'}(AvatarComponent);
  
function AvatarComponent(props: AvatarProps) {
  return <MyComponentLibraryAvatar url={props.data.avatarUrl} />
}
  `,
  schema: `
type Query {
  viewer: User
}

type User {
  first_name: String!
  last_name: String!
  avatar_url: String!
}
  `,
};

export default function HomepageFeatures() {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">
          <div className="col col--8 col--offset-2">
            <h2 className="text--center margin-bottom--xl">
              Isograph bundles your components with data.
              <br />
              That makes building stable, data-driven apps easy.
            </h2>
          </div>
        </div>
        <div className="row">
          {FeatureList.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}
        </div>
        <div className="row">
          <div className="col col--8 col--offset-2">
            <h2 className="text--center margin-top--xl">
              Watch the talk from GraphQL Conf
            </h2>
          </div>
        </div>
        <div className="row">
          <div className="col col--8 col--offset-2 margin-top--lg">
            <iframe
              width="100%"
              height="444"
              src="https://www.youtube-nocookie.com/embed/gO65JJRqjuc?si=cPnngBys86lgWeLA"
              title="YouTube video player"
              allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
              allowFullScreen
              // @ts-ignore
              frameborder="0"
            ></iframe>
          </div>
        </div>
        <div className="row">
          <div className="col col--8 col--offset-2 margin-top--xl">
            <h2 className="text--center">The basics</h2>
          </div>
          <div className="col col--8 margin-bottom--xl col--offset-2">
            <Tabs>
              <TabItem value="HomePageRoute" label="HomePageRoute.tsx">
                <p>
                  <code>HomePageRoute</code> is a regular ol' React component.
                  It makes a network request for the data needed by all the
                  components on the home page (no more, no less!). When that
                  request completes, it renders the home page.
                </p>
                <CodeBlock language="tsx">
                  {CodeBlocks.home_page.trim()}
                </CodeBlock>
              </TabItem>
              <TabItem value="HomePage" label="HomePage.tsx">
                <p>
                  <code>Query.HomePage</code> is an Isograph client-defined
                  field. The function HomePageComponent will be called with the
                  data selected in the <code>iso</code> literal.
                </p>
                <p>
                  In this <code>iso</code> literal, we select another
                  client-defined field: <code>avatar_component</code>.
                  Client-defined fields can reference each other, and must
                  eventually bottom out at server fields.
                </p>
                <CodeBlock language="tsx">
                  {CodeBlocks.home_page_component.trim()}
                </CodeBlock>
              </TabItem>
              <TabItem value="Avatar" label="Avatar.tsx">
                <p>
                  The code for this <code>User.Avatar</code> client-defined
                  field should be pretty familiar. The only thing to note is
                  that we're importing a regular ol' React component:{' '}
                  <code>Avatar</code>. That's allowed, too!
                </p>
                <CodeBlock language="tsx">
                  {CodeBlocks.avatar_component.trim()}
                </CodeBlock>
              </TabItem>
              <TabItem value="schema" label="Schema.graphql">
                <p>
                  Our GraphQL schema file includes all server fields that we
                  have accessed (such as <code>Query.viewer</code> and{' '}
                  <code>User.first_name</code>.) It does <b>not</b> include our
                  client-defined fields &mdash; the Isograph compiler
                  understands that <code>iso</code> literals <b>are</b> the
                  definition of client-defined fields.
                </p>
                <CodeBlock language="tsx">{CodeBlocks.schema.trim()}</CodeBlock>
              </TabItem>
            </Tabs>
          </div>
        </div>
      </div>
    </section>
  );
}
