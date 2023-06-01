import { bDeclare } from "@boulton/react";
import * as React from "react";

export const avatar_component = bDeclare`
  "An avatar"
  User.avatar_component {
    name,
    email,
    avatar_url,
  }
`((data) => {
  return (
    <div>
      <a href={`mailto:${data.email}`} alt={`email ${data.name}`}>
        <img src={data.avatar_url} />
      </a>
    </div>
  );
});
