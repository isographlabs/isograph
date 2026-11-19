import type { Link, Variables } from './store';

/** The default export of a `normalization_ast.ts` artifact, whose operation's root entity is
 * `TEntityName`. */
export type NormalizationAst<TEntityName extends EntityName = EntityName> = {
  /** The response key under which every composite response object holds its concrete
   * entity's name. */
  readonly discriminatorResponseKey: string;
  /** The record the top-level selections are written onto. */
  readonly root: NormalizationRootRecord<TEntityName>;
  readonly selections: NormalizationSelectionSet;
};

/** The operation's root entity's one record. */
export type NormalizationRootRecord<
  TEntityName extends EntityName = EntityName,
> = {
  readonly entityName: TEntityName;
  readonly id: string;
};

export type NormalizationSelectionSet = ReadonlyArray<NormalizationNode>;

export type NormalizationNode =
  | NormalizationSelection
  | NormalizationConditional;

/** A selection of a selectable. */
export type NormalizationSelection = {
  readonly kind: 'Selection';
  /** The alias the query text gives this selection, the key of its value in the response;
   * `null` when the query prints no alias and the key is `selectableName`. */
  readonly responseKey: string | null;
  readonly selectableName: string;
  readonly arguments: Arguments | null;
  /** `null` when the target entity is a leaf. */
  readonly target: NormalizationCompositeTarget | null;
};

/** The selections on a composite target entity, and which records its response objects are. */
export type NormalizationCompositeTarget = {
  readonly entity: NormalizationEntity;
  readonly identity: NormalizationIdentity;
  readonly selections: NormalizationSelectionSet;
};

/** Which entity a response object's record is of. */
export type NormalizationEntity =
  /** Every response object is of `entityName`. */
  | { readonly kind: 'Known'; readonly entityName: EntityName }
  /** The response object's value under `discriminatorResponseKey` names its entity. */
  | { readonly kind: 'Discriminator' };

/** How a response object's record id is found. */
export type NormalizationIdentity =
  /** The value under `selectableName`, an unaliased selection among the selection set's
   * `Selection` nodes. */
  | { readonly kind: 'Select'; readonly selectableName: string }
  /** Derived from the parent record and the path. */
  | { readonly kind: 'Omit' }
  /** Every response object of the entity is the record `id`. */
  | { readonly kind: 'Fixed'; readonly id: string };

/** Selections normalized onto the parent's record when `condition` holds. */
export type NormalizationConditional = {
  readonly kind: 'Conditional';
  readonly condition: Condition;
  /** The identity of the condition's entity. */
  readonly identity: NormalizationIdentity;
  readonly selections: NormalizationSelectionSet;
};

/** When a conditional's selections apply: the record's entity is `entityName`. */
export type Condition = {
  readonly kind: 'SelectionEquals';
  readonly entityName: EntityName;
};

/** An entity's name: GraphQL's type name, the value of `__typename`. */
export type EntityName = string;

export type Arguments = ReadonlyArray<Argument>;
export type Argument = readonly [string, ArgumentValue];

export type ArgumentValue =
  | { readonly kind: 'Variable'; readonly name: string }
  | { readonly kind: 'Literal'; readonly value: number | boolean | null }
  | { readonly kind: 'String'; readonly value: string }
  | { readonly kind: 'Object'; readonly value: Arguments }
  | { readonly kind: 'List'; readonly value: ReadonlyArray<ArgumentValue> };

/** The default export of an `entrypoint.ts` artifact: an entrypoint whose field's param type
 * is `TParam` and output type is `TOutput`, on the root entity `TEntityName`. */
export type IsographEntrypoint<
  TParam extends ResolverParam<TEntityName>,
  TOutput,
  TEntityName extends EntityName,
> = {
  readonly kind: 'Entrypoint';
  /** `<Entity>.<selectable>`. */
  readonly name: string;
  readonly queryText: string;
  readonly normalizationAst: NormalizationAst<TEntityName>;
  /** Returns the field's reader artifact. A function, so that loading this module does not
   * read the reader artifact's module, which imports the field's source file. */
  readonly readerArtifact: () => ReaderArtifact<TParam, TOutput, TEntityName>;
};

/**
 * What the default export of a `resolver_reader.ts` artifact returns; the default export is a
 * function, as upstream's (`() => ReaderArtifact<..>`), so that no import is read while the
 * module is evaluated. `TEntityName` is the name of the field's parent entity, or the union of
 * its concrete entities' names when it is abstract.
 */
export type ReaderArtifact<
  TParam extends ResolverParam<TEntityName>,
  TOutput,
  TEntityName extends EntityName,
> = {
  readonly readerAst: ReaderSelectionSet;
  /**
   * A method signature, so TypeScript compares its parameter bivariantly: an artifact
   * declared with one field's param type is also a
   * `ReaderArtifact<ResolverParam, TOutput, EntityName>`.
   */
  resolver(param: TParam): TOutput;
};

/** The variables of a field whose resolver parameter is `T`. */
export type ExtractParameters<T> = T extends {
  readonly parameters: infer P extends Variables;
}
  ? P
  : Variables;

/** What `read` calls a resolver with. */
export type ResolverParam<TEntityName extends EntityName = EntityName> = {
  readonly data: SelectionSetData;
  readonly parameters: Variables;
  /** The record the resolver's reader AST was read at. */
  readonly link: Link<TEntityName>;
};

/** The resolver's `data`: one value per reader node, under the node's `alias`. */
export type SelectionSetData = { readonly [alias: string]: unknown };

export type ReaderSelectionSet = ReadonlyArray<ReaderSelection>;

export type ReaderSelection =
  | ReaderScalarSelection
  | ReaderLinkedSelection
  | ReaderRefinementSelection
  | ReaderResolverSelection;

/** A selection of a server selectable whose target is a scalar: the stored value, whole. */
export type ReaderScalarSelection = {
  readonly kind: 'Scalar';
  readonly alias: string;
  readonly selectableName: string;
  readonly arguments: Arguments | null;
};

/** A selection of a server selectable whose target has selectables. */
export type ReaderLinkedSelection = {
  readonly kind: 'Linked';
  readonly alias: string;
  readonly selectableName: string;
  readonly arguments: Arguments | null;
  /** Read at every record the stored value links to. */
  readonly selections: ReaderSelectionSet;
};

/** A refinement `asT`: `selections` read on the same record when `condition` holds, else `null`. */
export type ReaderRefinementSelection = {
  readonly kind: 'Refinement';
  readonly alias: string;
  readonly condition: Condition;
  readonly selections: ReaderSelectionSet;
};

/** A client field selection: the value its resolver returns. */
export type ReaderResolverSelection = {
  readonly kind: 'Resolver';
  readonly alias: string;
  /** The values the selection passes to the field's variables, in terms of the reader's own. */
  readonly arguments: Arguments | null;
  /** The selected field's reader artifact's default export, called when the read reaches it. */
  readonly readerArtifact: () => ReaderArtifact<
    ResolverParam,
    unknown,
    EntityName
  >;
};
