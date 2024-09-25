import React, { Suspense, useRef } from 'react';
import { iso } from '@iso';
import { Button, Card, CardContent, Typography } from '@mui/material';
import { useOnScreen } from './useIntersection';
import {
  FragmentReader,
  useClientSideDefer,
  useImperativeLoadableField,
  useImperativeReference,
} from '@isograph/react';
import { UNASSIGNED_STATE } from '@isograph/react-disposable-state';

function capitalize(str: string) {
  return str.replace(/(^\w|\s\w)/g, (m) => m.toUpperCase());
}
export const BlogItem = iso(`
  field BlogItem.BlogItemDisplay @component {
    author
    title
    content
    BlogItemMoreDetail @loadable(lazyLoadArtifact: true)
    image {
      ImageDisplayWrapper
    }
  }
`)((
  { data: blogItem },
  { onVisible, index }: { onVisible: (() => void) | null; index: number },
) => {
  const isIntersectingRef = useRef(null);
  useOnScreen(isIntersectingRef, onVisible);

  const { fragmentReference, loadField } = useImperativeLoadableField(
    blogItem.BlogItemMoreDetail,
  );

  return (
    <Card variant="outlined" ref={isIntersectingRef}>
      <CardContent>
        {blogItem.image != null ? <blogItem.image.ImageDisplayWrapper /> : null}
        <Typography variant="h3">
          {index}. {capitalize(blogItem.title)}
        </Typography>

        <Typography variant="h4">by {capitalize(blogItem.author)}</Typography>
        {blogItem.content.split('\n').map((paragraph, index) => (
          <p key={index}>{paragraph}</p>
        ))}
        <Suspense fallback={<p>Loading more...</p>}>
          {fragmentReference !== UNASSIGNED_STATE ? (
            <FragmentReader fragmentReference={fragmentReference} />
          ) : (
            <Button variant="contained" onClick={() => loadField()}>
              Load more content...
            </Button>
          )}
        </Suspense>
      </CardContent>
    </Card>
  );
});

export const ImageDisplayWrapper = iso(`
  field Image.ImageDisplayWrapper @component {
    ImageDisplay @loadable(lazyLoadArtifact: true)
  }
`)(({ data: image }) => {
  const { fragmentReference } = useClientSideDefer(image.ImageDisplay);
  return (
    <Suspense fallback={null}>
      <FragmentReader fragmentReference={fragmentReference} />
    </Suspense>
  );
});
