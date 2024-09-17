import Buttons from './Buttons';

export default function () {
  return (
    <div className="padding-bottom--xl padding-top--xl alt-background">
      <div className="container">
        <div className="row">
          <div className="col">
            <div className="kicker">Ask your doctor</div>
            <h2 className="text--center margin-bottom--lg">
              Is Isograph right for&nbsp;me?
            </h2>
          </div>
        </div>

        <div className="row">
          <div className="col col--8 col--offset-2">
            <div className="text--center-wide">
              <p>
                Isograph is <b>alpha software</b> under active development. The
                APIs are likely to change substantially. The project is feature
                incomplete.
              </p>
              <p>
                Nonetheless, we stand behind the developer experience, and the
                foundations are solid. Isograph is a a good fit for certain side
                projects and internal tools. The Isograph team would encourage
                you to give it a try and provide feedback!
              </p>
              <p>
                Join the <a href="https://discord.gg/rjDDwvZR">Discord</a>.
                Follow the{' '}
                <a href="https://twitter.com/isographlabs">
                  official Twitter account
                </a>
                . Check out the{' '}
                <a href="https://github.com/isographlabs/isograph/issues">
                  open issues on GitHub
                </a>
                .{' '}
              </p>
              <p className="margin-bottom--lg">
                If you want to make substantial contributions on a fast-moving,
                ambitious project that is pushing the boundaries of what is
                possible with web development, Isograph is the project for you.
              </p>
              <Buttons />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
