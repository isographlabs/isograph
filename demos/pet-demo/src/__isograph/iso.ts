import type Mutation__SetTagline__entrypoint from "./Mutation/SetTagline/entrypoint";
import type Query__HomeRoute__entrypoint from "./Query/HomeRoute/entrypoint";
import type Query__PetDetailRoute__entrypoint from "./Query/PetDetailRoute/entrypoint";
import type Query__SpeciesRoute__entrypoint from "./Query/SpeciesRoute/entrypoint";
type Identity = <TClientField extends (...args: never[]) => unknown>(clientField: TClientField) => TClientField;
type WhitespaceCharacter = " " | "	" | "\n";
type Whitespace<In> = In extends `${WhitespaceCharacter}${infer In}` ? Whitespace<In> : In;
type MatchesWhitespaceAndString<TString extends string, T> = Whitespace<T> extends `${TString}${string}` ? T : never;
export function iso<T>(param: T & MatchesWhitespaceAndString<"field Mutation.SetTagline", T>): Identity;
export function iso<T>(param: T & MatchesWhitespaceAndString<"field Owner.OwnerBadge", T>): Identity;
export function iso<T>(param: T & MatchesWhitespaceAndString<"field Owner.displayName", T>): Identity;
export function iso<T>(param: T & MatchesWhitespaceAndString<"field Pet.PetBestFriendCard", T>): Identity;
export function iso<T>(param: T & MatchesWhitespaceAndString<"field Pet.PetCheckinsCard", T>): Identity;
export function iso<T>(param: T & MatchesWhitespaceAndString<"field Pet.PetStatsCard", T>): Identity;
export function iso<T>(param: T & MatchesWhitespaceAndString<"field Pet.PetSummaryCard", T>): Identity;
export function iso<T>(param: T & MatchesWhitespaceAndString<"field Pet.bestFriend", T>): Identity;
export function iso<T>(param: T & MatchesWhitespaceAndString<"field Pet.fullName", T>): Identity;
export function iso<T>(param: T & MatchesWhitespaceAndString<"field Pet.owner", T>): Identity;
export function iso<T>(param: T & MatchesWhitespaceAndString<"field Query.HomeRoute", T>): Identity;
export function iso<T>(param: T & MatchesWhitespaceAndString<"field Query.PetDetailRoute", T>): Identity;
export function iso<T>(param: T & MatchesWhitespaceAndString<"field Query.SpeciesRoute", T>): Identity;
export function iso<T>(param: T & MatchesWhitespaceAndString<"entity Owner", T>): void;
export function iso<T>(param: T & MatchesWhitespaceAndString<"entrypoint Mutation.SetTagline", T>): typeof Mutation__SetTagline__entrypoint;
export function iso<T>(param: T & MatchesWhitespaceAndString<"entrypoint Query.HomeRoute", T>): typeof Query__HomeRoute__entrypoint;
export function iso<T>(param: T & MatchesWhitespaceAndString<"entrypoint Query.PetDetailRoute", T>): typeof Query__PetDetailRoute__entrypoint;
export function iso<T>(param: T & MatchesWhitespaceAndString<"entrypoint Query.SpeciesRoute", T>): typeof Query__SpeciesRoute__entrypoint;
export function iso(_isographLiteralText: string): Identity | object | void {
  throw new Error("iso: Unexpected invocation at runtime. Either the Babel transform was not set up, or it failed to identify this call site. Make sure it is being used verbatim as `iso`.");
}
