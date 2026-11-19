/** The default export of a `normalization_ast.ts` artifact. */
export type NormalizationAst = {
  /** The response key under which every composite response object holds its concrete
   * entity's name. */
  readonly discriminatorResponseKey: string;
  readonly selections: NormalizationSelectionSet;
};

export type NormalizationSelectionSet = ReadonlyArray<NormalizationNode>;

export type NormalizationNode =
  | NormalizationSelection
  | NormalizationConditional;

/** A selection of a selectable. */
export type NormalizationSelection = {
  readonly kind: 'Selection';
  /** The alias the query text gives this selection: the key of its value in the response. */
  readonly responseKey: string;
  readonly selectableName: string;
  readonly arguments: Arguments | null;
  /** `null` when the target entity is a leaf. */
  readonly target: NormalizationCompositeTarget | null;
};

/** The selections on a composite target entity, and how its records are keyed. */
export type NormalizationCompositeTarget = {
  /** The response key of the target entity's identity selection, one of `selections`; `null`
   * when the entity has none, and the normalizer derives the record id. */
  readonly identityResponseKey: string | null;
  readonly selections: NormalizationSelectionSet;
};

/** Selections normalized onto the parent's record when `condition` holds. */
export type NormalizationConditional = {
  readonly kind: 'Conditional';
  readonly condition: NormalizationCondition;
  /** The response key of the condition entity's identity selection, one of `selections`;
   * `null` when the entity has none. */
  readonly identityResponseKey: string | null;
  readonly selections: NormalizationSelectionSet;
};

export type NormalizationCondition = {
  readonly kind: 'SelectionEquals';
  /** A selection every response object of the parent holds: the discriminator. */
  readonly responseKey: string;
  readonly value: string;
};

export type Arguments = ReadonlyArray<Argument>;
export type Argument = readonly [string, ArgumentValue];

export type ArgumentValue =
  | { readonly kind: 'Variable'; readonly name: string }
  | { readonly kind: 'Literal'; readonly value: number | boolean | null }
  | { readonly kind: 'String'; readonly value: string }
  | { readonly kind: 'Object'; readonly value: Arguments };
