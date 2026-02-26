export type Query__SmartestPetRoute__raw_response_type = {
  readonly pets: ReadonlyArray<{
    readonly id: string,
    readonly stats?: ({
      readonly intelligence?: (number | null),
    } | null),
  }>,
}

