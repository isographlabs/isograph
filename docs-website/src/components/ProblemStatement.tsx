export default function () {
  return (
    <div className="container margin-bottom--lg margin-top--xl">
      <div className="row">
        <div className="col">
          <div className="kicker">A tale as old as time</div>
          <h2 className="text--center margin-bottom--lg">
            Your engineering team is growing.
            <br />
            Are you ready?
          </h2>
        </div>
      </div>
      <div className="row margin-bottom--xl">
        <div className="col col--8 col--offset-2">
          <p>
            As your team grows, effective coordination and communication will
            become harder. No one will understand the full system, and no one
            will be able to guarantee that innocent-looking changes are safe to
            land. <b>Your app will be chronically unstable</b>.
          </p>
          <p>
            You might combat this by adding process and manual testing,{' '}
            <b>sacrificing developer velocity</b>.
          </p>
          <p>
            Or, maybe you'll restructure your app to make parts less
            interdependent. Each view will fetch its own data, leading to data
            being loaded redundantly and a jarring loading experience. Or your
            query declarations will be append-only.{' '}
            <b>Your app will slow to a crawl</b>.
          </p>
          <h3 className="text--center">
            Either way, your competitors will eat your lunch.
          </h3>
        </div>
      </div>
    </div>
  );
}
