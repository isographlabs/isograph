import type { Link } from '@isograph/react';
import type { StartUpdate } from '@isograph/react';

export type Query__linkedUpdate__param = {
  readonly data: {
    readonly node: ({
      /**
A client pointer for the Economist type.
      */
      readonly asEconomist: ({
        readonly name: string,
      } | null),
    } | null),
    readonly john_stuart_mill: ({
      /**
A store Link for the Node type.
      */
      readonly link: Link<"Node">,
      /**
A client pointer for the Economist type.
      */
      readonly asEconomist: ({
        readonly name: string,
      } | null),
    } | null),
  },
  readonly parameters: Record<PropertyKey, never>,
  readonly startUpdate: StartUpdate<{
    get node(): ({
      /**
A client pointer for the Economist type.
      */
      readonly asEconomist: ({
        name: string,
      } | null),
    } | null),
    set node(value: ({ link: Link<"Node"> } | null)),
    readonly john_stuart_mill: ({
      /**
A store Link for the Node type.
      */
      readonly link: Link<"Node">,
      /**
A client pointer for the Economist type.
      */
      readonly asEconomist: ({
        readonly name: string,
      } | null),
    } | null),
  }>,
};
