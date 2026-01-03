import { iso } from '@iso';

export const NewsfeedPaginationComponent = iso(`
  field Viewer.NewsfeedPaginationComponent(
    $skip: Int !
    $limit: Int !
  ) {
    newsfeed(
      skip: $skip
      limit: $limit
    ) {
      asAdItem {
        id
      }
      asBlogItem {
        id
      }
      NewsfeedAdOrBlog
    }
  }
`)(({ data }) => {
  return data.newsfeed;
});
