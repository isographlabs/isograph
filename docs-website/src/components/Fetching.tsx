import CodeBlock from './CodeBlock';

export default function () {
  return (
    <div className="padding-bottom--xl padding-top--xl">
      <div className="container">
        <div className="row">
          <div className="col">
            {/* <div className="kicker">Show me the money</div> */}
            <h2 className="text--center  margin-bottom--lg">Fetching data</h2>
          </div>
        </div>

        <div className="row">
          <div className="col col--5">
            <p>
              Those components aren't doing much on their own. Somehow, we need
              to make a network request for the data and render the{' '}
              <code>PetList</code> component.
            </p>
            <p>
              We start by writing <code>iso(`entrypoint Query.PetList`)</code>.
              This instructs the Isograph compiler to generate a query for all
              of the data needed by <code>Query.PetList</code> component, or any
              of its subcomponents.
            </p>
            <p>
              Next, we pass this to <code>useLazyReference</code>, which makes
              the network request when the <code>PetListRoute</code> component
              is rendered.
            </p>
            <p>
              This gives us an opaque query reference. We get the result of that
              request by calling <code>useResult</code>, and render that.
            </p>
          </div>
          <div className="col col--7">
            <CodeBlock language="tsx">{resolverDefinition.trim()}</CodeBlock>
          </div>
        </div>
      </div>
    </div>
  );
}

const resolverDefinition = `
import {iso} from '@iso';

export default function PetListRoute() {
  const {fragmentReference} = useLazyReference(
    iso(\`entrypoint Query.PetList\`),
    {}
  );

  const PetList = useResult(fragmentReference);

  return <PetList />;
}
`;
