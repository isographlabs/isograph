export type Query__meNameSuccessor__raw_response_type = {
  readonly me: {
    readonly id: string,
    readonly name: string,
    readonly successor?: ({
      readonly id: string,
      readonly successor?: ({
        readonly id: string,
        readonly name: string,
      } | null),
    } | null),
  },
}

