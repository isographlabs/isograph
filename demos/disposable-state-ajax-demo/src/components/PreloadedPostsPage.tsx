import React, { useEffect } from 'react';
import NoSSR from 'react-no-ssr';

import {
  UNASSIGNED_STATE,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import { makeNetworkRequest } from './api';
import { Comment, Post, User } from './networkTypes';
import { PromiseWrapper, useReadPromise } from './PromiseWrapper';
import { Card } from './Card';

/**
 * Preloading demo
 *
 * In this demo, we imperatively (i.e. not during render) make two network requests:
 * - We fetch the list of posts during a useEffect.
 * - We fetch the list of comments for a post when one mouses over a button.
 *
 * This demonstrates good and bad patterns!
 *
 * > Since this is a regular ol' REST endpoint, there is no way to combine the
 * > network request for the list of posts and the network request for a post's
 * > author, so we ignore posts' authors here. The best we can manage is to lazily
 * > load the post's author, which is demonstrated in the
 * > [lazy loading demo](./LazyLoadPostsPage.tsx).
 *
 * # The good
 *
 * We prefetch the comments for a post when you mouse over a button.
 * This is great! We can start the network request as soon as the user has signaled
 * intent to load a resource.
 *
 * Note that this is verbose and boilerplatey! In practice, library authors can
 * and should hide away much of this complexity.
 *
 * # The bad
 *
 * We fetch the list of posts during a useEffect.
 *
 * Note that fetching during an effect is **bad** and **has worse performance
 * than lazily loading**. However, we do not have router integration in this
 * demonstration, so the best we can do at the root of a page is to lazy load.
 * A future demo may add router integration.
 *
 * # The ugly
 *
 * In order to preload the comments for a given page, one must write
 * a decent amount of boilerplate. Note that for most cases, a library can
 * hide much of this complexity and only require that the developer use
 * react-disposable-state directly for more complicated use cases.
 *
 * This boilerplate is intentionally left in this demo!
 *
 * # Terminology
 *
 * - Lazy: we are using "lazy" here how Relay uses lazy, meaning the fetch occurs
 *   during the initial render.
 * - Reader: a component which reads a PromiseWrapper, and can suspend,
 *   but which does not trigger any network requests when rendered.
 * - Wrapper: a component which renders a suspense boundary and a PrelaodedLoader
 *   child component. The PromiseWrapper expected by the child is loaded
 *   imperatively (i.e. in response to an event).
 *
 * # Network config
 *
 * The loading states pass by very quickly! When testing this page in Chrome,
 * you can see the loading states more clearly if you create a custom network
 * config with added latency.
 */

/**
 * The PreloadedPostsWrapper component.
 * - It makes a network request in a useEffect. **This is strictly worse than
 *   making the network request lazily (i.e. during render)**.
 *   - The reason we are doing this is because we do not have router
 *     integration. A router should be making this API call for you, and passing
 *     the PromiseWrapper to the child component.
 */
export function PreloadedPostsWrapper() {
  const { state: requestForPosts, setState: setRequestForPosts } =
    useUpdatableDisposableState<PromiseWrapper<Post[]>>();
  useEffect(() => {
    setRequestForPosts(
      makeNetworkRequest('https://jsonplaceholder.typicode.com/posts'),
    );
  }, [setRequestForPosts]);

  if (requestForPosts === UNASSIGNED_STATE) {
    return <FullPageLoading />;
  }

  return (
    <NoSSR>
      <React.Suspense fallback={<FullPageLoading />}>
        <PreloadedPostsReader requestForPosts={requestForPosts} />
      </React.Suspense>
    </NoSSR>
  );
}

function PreloadedPostsReader({
  requestForPosts,
}: {
  requestForPosts: PromiseWrapper<Post[]>;
}) {
  const data = useReadPromise(requestForPosts);

  return (
    <>
      <h1 className="mt-5">Preloaded demo</h1>
      {data.map((post) => (
        <PostCard post={post} key={post.id} />
      ))}
    </>
  );
}

function PostCard({ post, user }: { post: Post; user?: User }) {
  return (
    <Card
      title={post.title}
      author={user != null ? `${user.name} (${user.email})` : null}
      body={
        <>
          {post.body}
          <CommentsWrapper postId={post.id} />
        </>
      }
    />
  );
}

/**
 * In order to avoid having multiple pieces of potentially-inconsistent
 * state, we define an enum here.
 *
 * This enum has two variants, but we're using it to handle three:
 * - Not loaded, not revealed
 * - Loaded, not revealed
 * - Loaded, revealed
 *
 * The state returned by the useUpdatableDisposableState hook originally
 * has the value UNASSIGNED_STATE. That signifies the not loaded, not
 * revealed state.
 */
type CommentsWrapperState =
  | {
      kind: 'LoadedNotRevealed';
      requestForComments: PromiseWrapper<Comment[]>;
    }
  | {
      kind: 'LoadedRevealed';
      requestForComments: PromiseWrapper<Comment[]>;
    };

function CommentsWrapper({ postId }: { postId: number }) {
  const { setState: setCommentsWrapperState, state: commentsWrapperState } =
    useUpdatableDisposableState<CommentsWrapperState>();

  function onMouseOver() {
    if (commentsWrapperState === UNASSIGNED_STATE) {
      const [networkRequest, cleanupNetworkRequest] = makeNetworkRequest<
        Comment[]
      >(`https://jsonplaceholder.typicode.com/post/${postId}/comments`);
      setCommentsWrapperState([
        {
          kind: 'LoadedNotRevealed',
          requestForComments: networkRequest,
        },
        cleanupNetworkRequest,
      ]);
    }
  }

  function onClick() {
    if (commentsWrapperState === UNASSIGNED_STATE) {
      const [networkRequest, cleanupNetworkRequest] = makeNetworkRequest<
        Comment[]
      >(`https://jsonplaceholder.typicode.com/post/${postId}/comments`);
      setCommentsWrapperState([
        {
          kind: 'LoadedRevealed',
          requestForComments: networkRequest,
        },
        cleanupNetworkRequest,
      ]);
    } else if (commentsWrapperState.kind === 'LoadedNotRevealed') {
      const { requestForComments } = commentsWrapperState;
      /**
       * Note: normally, here we would do
       * const [newRequestsForComments, cleanupNetworkRequest] =
       *   requestForComments.cloneIfNotDisposed()!;
       *
       * However, for the purposes of this demo, which does not use reference counted pointers
       * (this will be added in a future demo where it makes more sense), creating a dummy cleanup
       * function suffices.
       *
       * Note that we don't actually do anything during cleanup. See [./api.ts](./api.ts).
       *
       * TODO: create alternative APIs for resources that don't need cleanup, and use that API here!
       */
      const cleanupNetworkRequest = () => {};

      setCommentsWrapperState([
        { kind: 'LoadedRevealed', requestForComments },
        cleanupNetworkRequest,
      ]);
    }
  }

  if (
    commentsWrapperState === UNASSIGNED_STATE ||
    commentsWrapperState.kind === 'LoadedNotRevealed'
  ) {
    return (
      <div className="d-grid mt-2">
        <button
          className="btn btn-primary"
          onClick={onClick}
          onMouseOver={onMouseOver}
        >
          Fetch comments on hover, reveal comments on click
        </button>
      </div>
    );
  }

  return (
    <React.Suspense fallback={<h5 className="mt-2">Loading comments...</h5>}>
      <CommentsReader
        requestForComments={commentsWrapperState.requestForComments}
      />
    </React.Suspense>
  );
}

function CommentsReader({
  requestForComments,
}: {
  requestForComments: PromiseWrapper<Comment[]>;
}) {
  const comments = useReadPromise(requestForComments);

  return (
    <>
      <h5 className="mt-2">Comments</h5>
      <ul className="list-group">
        {comments.map((comment) => (
          <li className="list-group-item" key={comment.id}>
            {comment.email}: {comment.body}
          </li>
        ))}
      </ul>
    </>
  );
}

function FullPageLoading() {
  return <h1 className="mt-5">Loading...</h1>;
}
