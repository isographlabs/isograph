import Heading from "@theme/Heading";
import styles from "./styles.module.css";
import Tabs from "@theme/Tabs";
import TabItem from "@theme/TabItem";
import CodeBlock from "@theme/CodeBlock";

const FeatureList = [
  {
    title: "No boilerplate",
    description: (
      <>
        <div>
          Isograph eliminates the boilerplate of working with GraphQL. No more
          fussing with fragment references — Isograph does all of that for you.
        </div>
        <br />
        <div>
          All you need to do is to define a component and the fields it needs.
          Isograph will take care of the rest.
        </div>
      </>
    ),
  },
  {
    title: "Unparalleled performance",
    description: (
      <>
        <div>
          Great performance &mdash; like a single query per view that fetches
          exactly the right fields. That's all table stakes.
        </div>
        <br />
        <div>
          Isograph goes further, and improves your app's performance in ways
          that aren't possible in other frameworks.
        </div>
      </>
    ),
  },
  {
    title: "Tremendous stability",
    description: (
      <>
        <div>
          A component's dependencies do not affect the data that another
          component receives. Local reasoning is enough to ensure application
          stability, so engineers can feel safe moving fast.
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

export function HomePageRoute() {
  // Step 1: Make a network request (during render) for the
  // Query.home_page_component client-defined field.
  const { queryReference } = useLazyReference(
    // Note that calling isoFetch here **generates** a query
    // at compile time!
    isoFetch${"`"}Query.home_page_component${"`"},
    { /* Query variables */ }
  );
  
  // Step 2: Attempt to read the query reference. This will
  // return the value of the Query.home_page_component field
  // when the network request is complete.
  const HomePage = read(queryReference);

  // Step 3: Render the resulting component.
  // (In the future, <HomePage /> will be valid! For now,
  // you must call it as a function.)
  return HomePage({ /* render-time props */ });
}
  `,
  home_page_component: `
import { iso } from "@isograph/react";
import {
  ResolverParameterType as HomePageComponentParams,
} from "@iso/Query/home_page_component/reader.isograph";

// Step 1: Export the home_page_component and call iso
export const home_page_component = iso<
  // Step 2: Pass type parameters to iso. (This will not be
  // necessary soon.)
  HomePageComponentParams,
  ReturnType<typeof HomePageComponent>
>${"`"}
  // Step 3: Define a field named home_page_component on the
  // Query type, and tell the Isograph compiler that it is a
  // React @component
  Query.home_page_component @component {
    // Step 4: Select whatever fields you'll need, including
    // other client-defined fields like avatar_component.
    viewer {
      first_name,
      last_name,
      avatar_component,
    },
  }
${"`"}(HomePageComponent);

function HomePageComponent({ data }: HomePageComponentParams) {
  const viewer = data.viewer;
  return <>
    <h1>Hello {viewer?.first_name} {viewer?.last_name}!</h1>
    {data.avatar_component({})}
  </>
}
  `,
  avatar_component: `
import { iso } from "@isograph/react";
import {
  ResolverParameterType as AvatarProps,
} from "@iso/User/avatar/reader.isograph";
import Avatar from 'my-component-library';
  
export const avatar_component = iso<
  AvatarProps,
  ReturnType<typeof Avatar>,
>${"`"}
  User.avatar_component @component {
    avatarUrl,
  }
${"`"}(AvatarComponent);
  
function AvatarComponent(props: AvatarProps) {
  return <Avatar url={props.data.avatarUrl} />
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
          <div className="col col--8 col--offset-2 margin-top--xl">
            <h2 className="text--center">The basics</h2>
          </div>
          <div className="col col--8 margin-bottom--xl col--offset-2">
            <Tabs>
              <TabItem value="home_page" label="HomePageRoute.tsx">
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
              <TabItem
                value="home_page_component"
                label="home_page_component.tsx"
              >
                <p>
                  <code>home_page_component</code> is an Isograph client-defined
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
              <TabItem value="avatar_component" label="avatar_component.tsx">
                <p>
                  The code for this <code>avatar_component</code> client-defined
                  field should be pretty familiar. The only thing to note is
                  that we're importing a regular ol' React component:{" "}
                  <code>Avatar</code>. That's allowed, too!
                </p>
                <CodeBlock language="tsx">
                  {CodeBlocks.avatar_component.trim()}
                </CodeBlock>
              </TabItem>
              <TabItem value="schema" label="Schema.graphql">
                <p>
                  Our GraphQL schema defines all server fields that we have
                  accessed (such as <code>Query.viewer</code> and{" "}
                  <code>User.first_name</code>.) It does <b>not</b> define our
                  client-defined fields &mdash; the Isograph compiler
                  understands that
                  <code>iso</code> literals <b>are</b> the definition of
                  client-defined fields.
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
