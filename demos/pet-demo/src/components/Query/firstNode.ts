import { iso } from '@iso';

export const firstNode = iso(`
  pointer Query.firstNode to Pet {
    node(id: 0) {
      asPet {
        link
      }
    }
  }
`)(({ data }) => data.node?.asPet?.link);
