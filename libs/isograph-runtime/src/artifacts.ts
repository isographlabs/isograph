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
  | { readonly kind: 'Object'; readonly value: Arguments };

/** The default export of an `entrypoint.ts` artifact: an entrypoint whose field's param type
 * is `TParam` and output type is `TOutput`, on the root entity `TEntityName`. */
export type IsographEntrypoint<
  TParam,
  TOutput,
  TEntityName extends EntityName,
> = {
  readonly kind: 'Entrypoint';
  /** `<Entity>.<selectable>`. */
  readonly name: string;
  readonly queryText: string;
  readonly normalizationAst: NormalizationAst<TEntityName>;
  /** Typed by `TParam` and `TOutput` once reader artifacts exist. */
  readonly readerArtifact: unknown;
  /** Type-only, absent at runtime, as upstream's `~TRawResponseType`. */
  readonly '~param'?: TParam;
  /** Type-only, absent at runtime. */
  readonly '~output'?: TOutput;
};
