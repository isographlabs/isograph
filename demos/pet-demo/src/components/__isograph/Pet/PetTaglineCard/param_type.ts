import type { StartUpdate } from '@isograph/react';

export type Pet__PetTaglineCard__param = {
  readonly data: {
    readonly id: string,
    readonly tagline: string,
  },
  readonly parameters: Record<PropertyKey, never>,
  readonly startUpdate: StartUpdate<{
    readonly id: string,
    tagline: string,
  }>,
};
