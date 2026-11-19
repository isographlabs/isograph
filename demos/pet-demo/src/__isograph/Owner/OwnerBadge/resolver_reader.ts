import type { ReaderArtifact } from '@isograph/runtime';
import type { Owner__OwnerBadge__output_type } from './output_type';
import type { Owner__OwnerBadge__param } from './param_type';
import { OwnerBadge as resolver } from './../../../Owner';
import Owner__displayName__resolver_reader from './../displayName/resolver_reader';

const readerArtifact = (): ReaderArtifact<
  Owner__OwnerBadge__param,
  Owner__OwnerBadge__output_type,
  'Owner'
> => ({
  readerAst: [
    {
      kind: 'Resolver',
      alias: 'displayName',
      arguments: null,
      readerArtifact: Owner__displayName__resolver_reader,
    },
  ],
  resolver: resolver,
});
export default readerArtifact;
