export default function () {
  return (
    <div className="padding-bottom--xl padding-top--xl alt-background">
      <div className="container">
        <div className="row">
          <div className="col">
            {/* <div className="kicker">What can a poor boy do?</div> */}
            <h2 className="text--center margin-bottom--lg">
              With Isograph, there's no tradeoff between{' '}
              <br className="wide-br" />
              velocity, stability and&nbsp;performance
            </h2>
          </div>
        </div>

        <div className="row narrow-columns">
          <div className="col col--4">
            <div className="text--center-wide padding-horiz--md-wide">
              <h3>Local reasoning</h3>
              <p>
                Developers can make changes to individual files without
                reasoning about the rest of the app. If you alter a component,
                there's no need to update query declarations or modify other
                files to ensure that the data makes it to the component &mdash;
                the compiler does that&nbsp;for&nbsp;you.
              </p>
              <p>
                We lean into Isograph's compiler and into generated files so
                that you can focus on what matters&nbsp;&mdash;&nbsp;shipping
                features and iterating.
              </p>
            </div>
          </div>
          <div className="col col--4">
            <div className="text--center-wide padding-horiz--md-wide">
              <h3>Unparalleled performance</h3>
              <p>
                On every save, the Isograph compiler generates queries that
                provide exactly the data needed by a view. Say goodbye to under-
                and&nbsp;over-fetching.
              </p>
              <p>
                So as engineers add features or refactor components, no work is
                required to keep the&nbsp;app&nbsp;performant.
              </p>
              {/* TODO add section about granular re-rendering when it is implemented */}
            </div>
          </div>
          <div className="col col--4">
            <div className="text--center-wide padding-horiz--md-wide">
              <h3>Build with confidence</h3>
              <p>
                Isograph components are truly independent. Changes to one don't
                affect the data that other components receive, and there are no
                common files (such as loaders or query declarations) where all
                changes must be coordinated. This is what makes apps built with
                Isograph stable, even as the apps evolve&nbsp;over&nbsp;time.
              </p>
              <p>
                But Isograph goes further: it ensures that after mutations, all
                needed fields are refetched, meaning your app remains in
                a&nbsp;consistent&nbsp;state.
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
