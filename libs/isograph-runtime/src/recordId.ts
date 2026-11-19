import type {
  EntityName,
  NormalizationCompositeTarget,
  NormalizationConditional,
  NormalizationEntity,
  NormalizationIdentity,
  NormalizationSelection,
} from './artifacts';
import type { JsonObject, RecordId, StoreLink, Variables } from './store';
import { argumentChunks } from './storeKey';
import { ownValue } from './util';

/** Where a composite response object sits: what `recordLink` reads besides the object. */
export type RecordPosition = {
  /** The record the object is linked from. */
  readonly parent: StoreLink;
  readonly selection: NormalizationSelection;
  /** One index per list level, from the outermost; `[]` outside a list. */
  readonly listIndexes: ReadonlyArray<number>;
  /** Response keys and list indexes from the top of the response, for error messages. */
  readonly path: ReadonlyArray<string | number>;
};

/** The record of the composite response object `response`, a value of `position.selection`,
 * whose target is `target`. Throws on a missing discriminator or a non-string identity value. */
export function recordLink(
  response: JsonObject,
  target: NormalizationCompositeTarget,
  discriminatorResponseKey: string,
  position: RecordPosition,
  variables: Variables,
): StoreLink {
  const entityName = recordEntityName(
    response,
    target.entity,
    discriminatorResponseKey,
    position,
  );
  return {
    entityName,
    id: recordId(
      response,
      recordIdentity(target, entityName),
      position,
      variables,
    ),
  };
}

function recordEntityName(
  response: JsonObject,
  entity: NormalizationEntity,
  discriminatorResponseKey: string,
  position: RecordPosition,
): EntityName {
  switch (entity.kind) {
    case 'Known':
      return entity.entityName;
    case 'Discriminator': {
      const value = ownValue(response, discriminatorResponseKey);
      if (typeof value !== 'string') {
        throw new Error(
          `Malformed response at ${JSON.stringify(position.path)}: expected a string under "${discriminatorResponseKey}".`,
        );
      }
      return value;
    }
  }
}

/** The identity of the conditional for `entityName` among `target`'s direct children, unless
 * it is `Omit`; otherwise the target's. */
function recordIdentity(
  target: NormalizationCompositeTarget,
  entityName: EntityName,
): NormalizationIdentity {
  const conditional = target.selections.find(
    (node): node is NormalizationConditional =>
      node.kind === 'Conditional' && node.condition.entityName === entityName,
  );
  return conditional !== undefined && conditional.identity.kind !== 'Omit'
    ? conditional.identity
    : target.identity;
}

function recordId(
  response: JsonObject,
  identity: NormalizationIdentity,
  position: RecordPosition,
  variables: Variables,
): RecordId {
  switch (identity.kind) {
    case 'Fixed':
      return identity.id;
    case 'Select': {
      const value = ownValue(response, identity.selectableName);
      if (value === undefined) {
        throw new Error(
          `Malformed response at ${JSON.stringify(position.path)}: expected the response key "${identity.selectableName}".`,
        );
      }
      if (typeof value !== 'string') {
        throw new Error(
          `Malformed response at ${JSON.stringify(position.path)}: expected a string record id under "${identity.selectableName}", got ${JSON.stringify(value)}.`,
        );
      }
      return value;
    }
    case 'Omit':
      return (
        `${position.parent.entityName}:${position.parent.id}.${position.selection.selectableName}` +
        position.listIndexes.map((index) => `.${index}`).join('') +
        argumentChunks(position.selection.arguments, variables)
      );
  }
}
