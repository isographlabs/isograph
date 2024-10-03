import React, { Suspense, useState } from 'react';
import { iso } from '@iso';
import { Card, CardContent, Container, Stack, Typography } from '@mui/material';
import {
  FragmentReader,
  useLazyReference,
  useSkipLimitPagination,
} from '@isograph/react';
import { FullPageLoading } from '../routes';
import { ErrorBoundary } from '../ErrorBoundary';

export const Newsfeed = iso(`
  field Query.Newsfeed @component {
    viewer {
      newsfeed(skip: 0, limit: 6) {
        NewsfeedAdOrBlog
      }
      NewsfeedPaginationComponent @loadable
    }
  }
`)(function PetDetailRouteComponent({ data }) {
  const viewer = data.viewer;

  const paginationState = useSkipLimitPagination(
    viewer.NewsfeedPaginationComponent,
  );

  const newsfeedItems = viewer.newsfeed.concat(paginationState.results);

  const loadMore = () => {
    if (paginationState.kind === 'Complete') {
      paginationState.fetchMore({}, 4);
    }
  };

  return (
    <Container maxWidth="md">
      <Stack direction="column" spacing={4} sx={{ pt: 8 }}>
        <Typography variant="h2">Lorem ipsum dolor sit amet</Typography>
        {newsfeedItems.map((newsfeedItem, index) => {
          const onVisible =
            index === newsfeedItems.length - 1 ? loadMore : null;
          return (
            // the key is not needed here!
            // eslint-disable-next-line react/jsx-key
            <newsfeedItem.NewsfeedAdOrBlog
              onVisible={onVisible}
              index={index}
            />
          );
        })}
      </Stack>
    </Container>
  );
});

export function NewsfeedLoader() {
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Query.Newsfeed`),
    {},
  );

  return (
    <ErrorBoundary>
      <React.Suspense fallback={<FullPageLoading />}>
        <FragmentReader
          fragmentReference={fragmentReference}
          networkRequestOptions={{ suspendIfInFlight: false }}
        />
      </React.Suspense>
    </ErrorBoundary>
  );
}

export const NewsfeedAdOrBlog = iso(`
  field NewsfeedItem.NewsfeedAdOrBlog @component {
    adItem {
      AdItemDisplayWrapper
    }
    blogItem {
      BlogItemDisplay
    }
  }
`)((
  { data: newsfeedItem },
  { onVisible, index }: { onVisible: (() => void) | null; index: number },
) => {
  const fallback = (
    <Card variant="outlined">
      <CardContent>Loading...</CardContent>
    </Card>
  );
  if (newsfeedItem.adItem != null) {
    return (
      <Suspense fallback={fallback}>
        <newsfeedItem.adItem.AdItemDisplayWrapper
          onVisible={onVisible}
          index={index}
        />
      </Suspense>
    );
  } else if (newsfeedItem.blogItem != null) {
    // Why is BlogItem not fetched loadably, but AdItemDisplayWrapper is?
    // This is because there is currently a limitation in Isograph:
    // you cannot fetch the *data* for a client field as part of the parent
    // query while imperatively loading the client field's JavaScript.
    //
    // So, the only thing that is currently possible is to fetch each blog
    // item in a separate newtork request, just so that we can load the JS
    // only when needed.
    //
    // Fixing this is a priority, but will take some work.
    return (
      <newsfeedItem.blogItem.BlogItemDisplay
        onVisible={onVisible}
        index={index}
      />
    );
  }
});
