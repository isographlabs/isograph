import { expectTypeOf, test } from 'vitest';
import nodeLinkReaderArtifact from '../fixtures/store/src/__isograph/Node/NodeLink/resolver_reader';
import petLinkReaderArtifact from '../fixtures/store/src/__isograph/Pet/PetLink/resolver_reader';
import linksReaderArtifact from '../fixtures/store/src/__isograph/Query/Links/resolver_reader';
import {
  type Link,
  read,
  type ReadResult,
  type Store,
  type StoreLink,
} from '../src/index';
import { queryRoot } from './fixtures';

const store: Store = new Map();
const petLink: Link<'Pet'> = { entityName: 'Pet', id: '1' };
const storeLink: StoreLink = petLink;

test('read takes a link to a record of the reader artifact’s parent entity', () => {
  read(store, linksReaderArtifact(), queryRoot, { id: '1' });
  read(store, nodeLinkReaderArtifact(), petLink, {});
  expectTypeOf(read(store, petLinkReaderArtifact(), petLink, {})).toEqualTypeOf<
    ReadResult<Link<'Pet'>>
  >();
  // @ts-expect-error a Pet link to a Query reader artifact
  read(store, linksReaderArtifact(), petLink, { id: '1' });
  // @ts-expect-error a Query link to a Pet reader artifact
  read(store, petLinkReaderArtifact(), queryRoot, {});
  // @ts-expect-error a Query link to a reader artifact of Node, which Query does not implement
  read(store, nodeLinkReaderArtifact(), queryRoot, {});
  // @ts-expect-error a link whose entity name is any string
  read(store, petLinkReaderArtifact(), storeLink, {});
});
