import { iso } from './__isograph/iso';

export const Feed = iso(`
  field Query.Feed($limit: Int!) {
    feed(limit: $limit) {
      asBlogItem {
        id
        title
        author {
          id
          name
        }
      }
      asAdItem {
        advertiser
      }
    }
  }
`)(function Feed({ data }) {
  return data;
});

iso(`entrypoint Query.Feed`);
