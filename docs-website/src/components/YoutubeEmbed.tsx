export default function YoutubeEmbed() {
  return (
    <section>
      <div className="container padding-bottom--xl padding-top--xl">
        <div className="row">
          <div className="col col--8 col--offset-2">
            {/* <div className="kicker">Tell me more!</div> */}
            <h2 className="text--center">
              Watch the talk at GraphQL&nbsp;Conf
            </h2>
          </div>
        </div>
        <div className="row">
          <div className="col col--8 col--offset-2 margin-top--md">
            <iframe
              width="100%"
              height="444"
              src="https://www.youtube-nocookie.com/embed/gO65JJRqjuc?si=cPnngBys86lgWeLA"
              title="YouTube video player"
              allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
              allowFullScreen
              frameBorder="0"
            ></iframe>
          </div>
        </div>
      </div>
    </section>
  );
}
