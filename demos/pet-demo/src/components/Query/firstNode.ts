import { iso } from '@iso';

export const firstNode = iso(`
  pointer Query.firstNode to Node {
    node(id: 0) {
      link
    }
  }
`)(({ data }) => data.node?.link);
