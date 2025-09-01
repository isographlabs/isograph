import type { Link } from '@isograph/react';

export type Query__smartestPet__param = {
  readonly data: {
    readonly pets: ReadonlyArray<{
      /**
A store Link for the Pet type.
      */
      readonly link: Link,
      readonly stats: ({
        readonly intelligence: (number | null),
      } | null),
    }>,
  },
  readonly parameters: Record<PropertyKey, never>,
};
