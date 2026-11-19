import type { ReaderArtifact } from '@isograph/runtime';
import type { Owner__displayName__output_type } from './output_type';
import type { Owner__displayName__param } from './param_type';
import { displayName as resolver } from './../../../Owner';

const readerArtifact = (): ReaderArtifact<
  Owner__displayName__param,
  Owner__displayName__output_type,
  'Owner'
> => ({
  readerAst: [],
  resolver: resolver,
});
export default readerArtifact;
