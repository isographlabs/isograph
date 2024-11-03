export default function YoutubeEmbed() {
  return (
    <section>
      <div className="container padding-bottom--xl padding-top--xl">
        <div className="row">
          <div className="col col--8 col--offset-2">
            <h2 className="text--center">
              Watch the talk at GraphQL&nbsp;Conf 2024
            </h2>
          </div>
        </div>
        <div className="row">
          <div className="col col--8 col--offset-2 margin-top--md">
            <iframe
              width="100%"
              height="444"
              src="https://www.youtube-nocookie.com/embed/sf8ac2NtwPY?si=Ztxho3m1mPKFO6ME"
              title="Performing Impossible Feats with Isograph at GraphQL Conf 2024"
              allow="autoplay; clipboard-write; encrypted-media; picture-in-picture; web-share"
              allowFullScreen
              frameBorder="0"
            ></iframe>
          </div>
        </div>
      </div>
    </section>
  );
}
