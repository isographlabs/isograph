import type { Link } from '@isograph/react';

export type NodeLinkFutureType = Link<"%Node future type%">;

export type NodeLink = 
  | Link<"Economist">
  | NodeLinkFutureType;
