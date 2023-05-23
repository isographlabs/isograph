import {bDeclare} from '@boulton/react';
import * as React from 'react';

export const current_post_component = bDeclare`
  "Show the current post, mmmkday"
  Query.current_post_component {
    current_post {
      name,
      content,
      author {
        name,
        avatar,
      },
    },
  }
`(data => {
  return (
    <div>
      <h1>{data.name}</h1>
      By <b>{data.author.name}</b>
      <p>{data.author.avatar}</p>
      <p>{data.content}</p>
    </div>
  );
})

