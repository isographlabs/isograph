import { type Node__link__output_type } from '../../Node/link/output_type';
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
      readonly link: Node__link__output_type,
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
    set node(value: ({ link: Node__link__output_type } | null)),
    readonly john_stuart_mill: ({
      /**
A store Link for the Node type.
      */
      readonly link: Node__link__output_type,
      /**
A client pointer for the Economist type.
      */
      readonly asEconomist: ({
        readonly name: string,
      } | null),
    } | null),
  }>,
};
