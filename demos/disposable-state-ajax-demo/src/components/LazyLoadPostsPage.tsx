import React, { useState } from 'react';
import NoSSR from 'react-no-ssr';

import { useLazyDisposableState } from '@isograph/react-disposable-state';
import { getOrCreateCacheForUrl } from './api';
import { Comment, Post, User } from './networkTypes';
import { useReadPromise } from './PromiseWrapper';
import { Card } from './Card';

/**
 * Lazy loading demo
 *
 * In this demo, we lazily (i.e. during render) make three network requests:
 *   - the list of posts
 *   - the post's author
 *   - the post's comments
 *
 * In all cases, we wrap the LazyLoader component (which is the one that
 * will fetch during render) in a Wrapper component, which is one that
 * renders a Suspense boundary.
 *
 * See [./PreloadedPostsPage](./PreloadedPostsPage.tsx) for a demonstration
 * of preloading the data imperatively.
 *
 * # Terminology
 *
 * - Lazy: we are using "lazy" here how Relay uses lazy, meaning the fetch occurs
 *   during the initial render.
 * - LazyLoader: a component which, when rendered, can cause a network request to
 *   occur. It can also suspend.
 * - Wrapper: a component which renders a suspense boundary and a LazyLoader child
 *   component.
 *
 * # Network config
 *
 * The loading states pass by very quickly! When testing this page in Chrome,
 * you can see the loading states more clearly if you create a custom network
 * config with added latency.
 */

/**
 * The LazyLoadPostsWrapper component. It:
 * - renders a suspense boundary and the child, which will suspend
 * - opt out of SSR. Otherwise, the suspense for the PostsPageLazyLoader happens
 *   on the server and we don't see any loading states!
 */
export function LazyLoadPostsWrapper() {
  return (
    <NoSSR>
      <React.Suspense fallback={<FullPageLoading />}>
        <PostsLazyLoader />
      </React.Suspense>
    </NoSSR>
  );
}

function PostsLazyLoader() {
  const cache = getOrCreateCacheForUrl<Post[]>(
    'https://jsonplaceholder.typicode.com/posts',
  );
  const apiCall = useLazyDisposableState(cache);
  const data = useReadPromise(apiCall.state);

  return (
    <>
      <h1 className="mt-5">Lazy loaded demo</h1>
      {data.map((post) => (
        <PostWrapper post={post} key={post.id} />
      ))}
    </>
  );
}

function PostWrapper({ post }: { post: Post }) {
  return (
    <React.Suspense fallback={<PostCard post={post} />}>
      <PostLazyLoader post={post} />
    </React.Suspense>
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

function PostLazyLoader({ post }: { post: Post }) {
  const cache = getOrCreateCacheForUrl<User>(
    `https://jsonplaceholder.typicode.com/users/${post.userId}`,
  );
  const apiCall = useLazyDisposableState(cache);
  const user = useReadPromise(apiCall.state);

  return <PostCard post={post} user={user} />;
}

function CommentsWrapper({ postId }: { postId: number }) {
  const [showComments, setShowComments] = useState(false);

  if (!showComments) {
    return (
      <div className="d-grid mt-2">
        <button
          className="btn btn-primary"
          onClick={() => setShowComments(true)}
        >
          Fetch comments using render-as-you-fetch
        </button>
      </div>
    );
  }

  return (
    <React.Suspense fallback={<h5 className="mt-2">Loading comments...</h5>}>
      <CommentsLazyLoader postId={postId} />
    </React.Suspense>
  );
}

function CommentsLazyLoader({ postId }: { postId: number }) {
  const cache = getOrCreateCacheForUrl<Comment[]>(
    `https://jsonplaceholder.typicode.com/post/${postId}/comments`,
  );
  const apiCall = useLazyDisposableState(cache);
  const comments = useReadPromise(apiCall.state);

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
