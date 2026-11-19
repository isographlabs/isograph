import type { ReaderArtifact } from '@isograph/runtime';
import type { Node__NodeLink__output_type } from './output_type';
import type { Node__NodeLink__param } from './param_type';
import { NodeLink as resolver } from './../../../NodeLink';

const readerArtifact = (): ReaderArtifact<
  Node__NodeLink__param,
  Node__NodeLink__output_type,
  'BlogItem' | 'Checkin' | 'Pet'
> => ({
  readerAst: [],
  resolver: resolver,
});
export default readerArtifact;
