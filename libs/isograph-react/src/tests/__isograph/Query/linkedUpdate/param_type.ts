import type { NodeLink } from '../../Node/link_type.ts';
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
      readonly link: NodeLink,
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
    set node(value: ({ link: NodeLink } | null)),
    readonly john_stuart_mill: ({
      /**
A store Link for the Node type.
      */
      readonly link: NodeLink,
      /**
A client pointer for the Economist type.
      */
      readonly asEconomist: ({
        readonly name: string,
      } | null),
    } | null),
  }>,
};
