import {bDeclare} from '@boulton/react';
import * as React from 'react';

export const header_component = bDeclare`
  "Header"
  Query.header_component {
    current_user {
      name,
      email,
      avatar_component,
    },
  }
`(data => {
  return (
    <div>
      <h1>Currently logged in as:</h1>
      <p>{data.current_user.name}</p>
      <p>{data.current_user.email}</p>
      <p>{data.current_user.avatar_component}</p>
    </div>
  );
})

