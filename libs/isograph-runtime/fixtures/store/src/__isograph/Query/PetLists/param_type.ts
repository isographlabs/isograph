export type Query__PetLists__param = {
  readonly data: {
    readonly pets: ReadonlyArray<{
      readonly id: string;
      readonly name: string;
    }>;
    readonly maybePets: (ReadonlyArray<({
      readonly id: string;
      readonly name: string;
    } | null)> | null);
    readonly petGrid: (ReadonlyArray<(ReadonlyArray<({
      readonly id: string;
      readonly name: string;
    } | null)> | null)> | null);
    readonly statGrid: (ReadonlyArray<(ReadonlyArray<({
      readonly weight: (number | null);
    } | null)> | null)> | null);
  };
  readonly parameters: Record<PropertyKey, never>;
};
