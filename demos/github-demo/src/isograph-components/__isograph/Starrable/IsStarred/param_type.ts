
import { type Variables } from '@isograph/react';

export type Starrable__IsStarred__param = {
  readonly data: {
        /**
Returns a count of how many stargazers there are on this object
    */
readonly stargazerCount: number,
        /**
Returns a boolean indicating whether the viewing user has starred this starrable.
    */
readonly viewerHasStarred: boolean,
  },
  readonly parameters: Variables,
};
