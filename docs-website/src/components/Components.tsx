import CodeBlock from './CodeBlock';

export default function () {
  return (
    <div className="padding-bottom--xl padding-top--xl alt-background">
      <div className="container">
        <div className="row">
          <div className="col">
            <div className="kicker">I'm ready to learn more</div>
            <h2 className="text--center  margin-bottom--lg">
              Defining Isograph components
            </h2>
          </div>
        </div>

        <div className="row">
          <div className="col col--5">
            <p>
              Isograph components are defined using an <code>iso</code> literal,
              in which we tell the Isograph compiler the name of the component (
              <code>PetList</code>) and the type on which that component can be
              accessed (the GraphQL type <code>Query</code>).
            </p>
            <p>
              We also specify the data that that component requires (e.g.{' '}
              <code>pets</code> and <code>id</code>). Dependencies on other
              components, such as <code>PetProfile</code>, are declared in the
              same way!
            </p>
            <p>
              To this <code>iso</code> literal, we pass the actual component
              that will be rendered. This is just a plain ol' React component,
              and all of the React features you would expect (e.g. state,
              context and suspense) work here.
            </p>
            <h3>Subcomponents</h3>
            <p>
              A typical Isograph app will be composed of many layers of Isograph
              components, before it bottoms out at server fields that are
              defined in a GraphQL schema.
            </p>
            <p>
              In this example, <code>PetProfile</code> is another Isograph
              component. In <code>PetList</code>, we render it directly, and we
              don't pass anything down to it &mdash; even though the{' '}
              <code>PetProfile</code> component itself requires data (
              <code>name</code> and <code>species</code>)!
            </p>
          </div>
          <div className="col col--7">
            <CodeBlock className="bordered-codeblock" language="tsx">
              {resolverDefinition.trim()}
            </CodeBlock>
          </div>
        </div>
      </div>
    </div>
  );
}

const resolverDefinition = `
import {iso} from '@iso';

export const PetList = iso(\`
  field Query.PetList @component {
    pets {
      id
      PetProfile
    }
  }
\`)(function(props) {
  return (<>
    <h1>Pet Hotel Guest List</h1>
    <p>{props.data.pets.length} pets checked in.</p>
    {props.data.pets.map(pet => (
      <pet.PetProfile key={pet.id} />
    ))}
  </>);
});

export const PetProfile = iso(\`
  field Pet.PetProfile @component {
    name
    species
  }
\`)(function(props) {
  return (<Box>
    {props.data.name}, a {props.data.species}
  </Box>);
});
`;
