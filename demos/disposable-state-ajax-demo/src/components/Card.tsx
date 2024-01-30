import React, { ReactNode } from 'react';

/**
 * The root component for posts. It lazily loads all posts, and renders them.
 */
export function Card({
  title,
  body,
  author,
}: {
  title: ReactNode;
  body: ReactNode;
  author: ReactNode | null;
}) {
  return (
    <CardChrome>
      <h4 className="card-title">{title}</h4>
      {author != null ? <h5 className="card-title">{author}</h5> : null}
      {body}
    </CardChrome>
  );
}

function CardChrome({ children }: { children: ReactNode }) {
  return (
    <div className="card mb-3">
      <div className="card-body">{children}</div>
    </div>
  );
}
