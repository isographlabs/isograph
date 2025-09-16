import type { Link } from '@isograph/react';

export type Query__firstNode__param = {
  readonly data: {
    readonly node: ({
      /**
A store Link for the Node type.
      */
      readonly link: Link<"Viewer"> | Link<"Pet"> | Link<"BlogItem"> | Link<"Image"> | Link<"AdItem">,
    } | null),
  },
  readonly parameters: Record<PropertyKey, never>,
};
