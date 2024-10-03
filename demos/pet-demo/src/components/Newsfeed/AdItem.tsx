import React, { useRef, useState } from 'react';
import { iso } from '@iso';
import { Card, CardContent, Typography } from '@mui/material';
import { useOnScreen } from './useIntersection';

function capitalize(str: string) {
  return str.replace(/(^\w|\s\w)/g, (m) => m.toUpperCase());
}
export const BlogItem = iso(`
  field AdItem.AdItemDisplay @component {
    advertiser
    message
  }
`)((
  { data: adItem },
  { onVisible, index }: { onVisible: (() => void) | null; index: number },
) => {
  const isIntersectingRef = useRef(null);
  useOnScreen(isIntersectingRef, onVisible);

  return (
    <Card variant="outlined" ref={isIntersectingRef}>
      <CardContent>
        <Typography variant="h3">
          {index}. {capitalize(adItem.message)}
        </Typography>
        <Typography variant="h4">
          Brought to you by {capitalize(adItem.advertiser)}
        </Typography>
        <small>This is sponsored content.</small>
      </CardContent>
    </Card>
  );
});
