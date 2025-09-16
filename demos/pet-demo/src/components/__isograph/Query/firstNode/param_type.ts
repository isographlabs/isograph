import type { Link } from '@isograph/react';

export type Query__firstNode__param = {
  readonly data: {
    readonly node: ({
      /**
A client pointer for the Pet type.
      */
      readonly asPet: ({
        /**
A store Link for the Pet type.
        */
        readonly link: Link<"Pet">,
      } | null),
    } | null),
  },
  readonly parameters: Record<PropertyKey, never>,
};
