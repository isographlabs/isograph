import React from 'react';
import { iso } from '@iso';
import { FragmentReader, useClientSideDefer } from '@isograph/react';

export const AdItemDisplayWrapper = iso(`
  field AdItem.AdItemDisplayWrapper @component {
    AdItemDisplay @loadable(lazyLoadArtifact: true)
  }
`)((
  { data },
  { onVisible, index }: { onVisible: (() => void) | null; index: number },
) => {
  const { fragmentReference } = useClientSideDefer(data.AdItemDisplay);

  return (
    <FragmentReader
      fragmentReference={fragmentReference}
      additionalProps={{ onVisible, index }}
    />
  );
});
