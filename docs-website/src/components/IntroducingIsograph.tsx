export default function IntroducingIsograph() {
  return (
    <div className="padding-bottom--xl padding-top--xl alt-background-2">
      <div className="container">
        <div className="row">
          <div className="col">
            {/* <div className="kicker">Who will save us?</div> */}
            <h1 className="text--center margin-bottom--lg why-built">
              That's why we built{' '}
              <span className="isograph-name">Isograph</span>
            </h1>
          </div>
        </div>
        <div className="row">
          <div className="col col--8 col--offset-2">
            <p className="text--center-wide margin-bottom--lg callout-1">
              Isograph is an opinionated framework for building interactive,
              data-driven apps. It&nbsp;makes heavy use of its compiler and of
              generated code to enable developers to quickly and confidently
              build stable and performant apps, while providing an amazing
              developer experience.
            </p>
            <p className="text--center-wide callout-2">
              Developers define components and the data they need.
              Isograph&nbsp;takes&nbsp;care&nbsp;of&nbsp;the&nbsp;rest.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
