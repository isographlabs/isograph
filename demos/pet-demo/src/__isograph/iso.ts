import { ResolverParameterType as field_Pet____refetch } from './Pet/__refetch/reader.ts'
import { ResolverParameterType as field_Checkin____refetch } from './Checkin/__refetch/reader.ts'
import { ResolverParameterType as field_Pet____set_pet_tagline } from './Pet/__set_pet_tagline/reader.ts'
import { ResolverParameterType as field_Pet____set_pet_best_friend } from './Pet/__set_pet_best_friend/reader.ts'
import { ResolverParameterType as field_Pet__PetBestFriendCard } from './Pet/PetBestFriendCard/reader.ts'
import { ResolverParameterType as field_Pet__PetTaglineCard } from './Pet/PetTaglineCard/reader.ts'
import { ResolverParameterType as field_Pet__PetUpdater } from './Pet/PetUpdater/reader.ts'
import { ResolverParameterType as field_Pet__PetCheckinsCard } from './Pet/PetCheckinsCard/reader.ts'
import { ResolverParameterType as field_Query__PetDetailRoute } from './Query/PetDetailRoute/reader.ts'
import { ResolverParameterType as field_Query__HomeRoute } from './Query/HomeRoute/reader.ts'
import { ResolverParameterType as field_Pet__PetPhraseCard } from './Pet/PetPhraseCard/reader.ts'
import { ResolverParameterType as field_Pet__PetSummaryCard } from './Pet/PetSummaryCard/reader.ts'
import { ResolverParameterType as field_Pet__PetStatsCard } from './Pet/PetStatsCard/reader.ts'
import entrypoint_Query__HomeRoute from '../__isograph/Query/HomeRoute/entrypoint.ts'
import entrypoint_Query__PetDetailRoute from '../__isograph/Query/PetDetailRoute/entrypoint.ts'
type IdentityWithParam<TParam> = <TResolverReturn>(
    x: (param: TParam) => TResolverReturn
) => (param: TParam) => TResolverReturn;

type WhitespaceCharacter = ' ' | '\n';
type Whitespace<In> = In extends `${WhitespaceCharacter}${infer In}`
? Whitespace<In>
: In;

type MatchesWhitespaceAndString<
TString extends string,
T
> = Whitespace<T> extends `${TString}${string}` ? T : never;export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Pet.__refetch', T>
        ): IdentityWithParam<field_Pet____refetch>;
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Checkin.__refetch', T>
        ): IdentityWithParam<field_Checkin____refetch>;
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Pet.__set_pet_tagline', T>
        ): IdentityWithParam<field_Pet____set_pet_tagline>;
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Pet.__set_pet_best_friend', T>
        ): IdentityWithParam<field_Pet____set_pet_best_friend>;
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Pet.PetBestFriendCard', T>
        ): IdentityWithParam<field_Pet__PetBestFriendCard>;
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Pet.PetTaglineCard', T>
        ): IdentityWithParam<field_Pet__PetTaglineCard>;
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Pet.PetUpdater', T>
        ): IdentityWithParam<field_Pet__PetUpdater>;
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Pet.PetCheckinsCard', T>
        ): IdentityWithParam<field_Pet__PetCheckinsCard>;
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Query.PetDetailRoute', T>
        ): IdentityWithParam<field_Query__PetDetailRoute>;
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Query.HomeRoute', T>
        ): IdentityWithParam<field_Query__HomeRoute>;
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Pet.PetPhraseCard', T>
        ): IdentityWithParam<field_Pet__PetPhraseCard>;
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Pet.PetSummaryCard', T>
        ): IdentityWithParam<field_Pet__PetSummaryCard>;
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Pet.PetStatsCard', T>
        ): IdentityWithParam<field_Pet__PetStatsCard>;
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'entrypoint Query.HomeRoute', T>
        ): IdentityWithParam<typeof entrypoint_Query__HomeRoute>;
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'entrypoint Query.PetDetailRoute', T>
        ): IdentityWithParam<typeof entrypoint_Query__PetDetailRoute>;

export function iso(_queryText: string): IdentityWithParam<any> {
  // The name `identity` here is a bit of a double entendre.
  // First, it is the identity function, constrained to operate
  // on a very specific type. Thus, the value of b Declare`...`(
  // someFunction) is someFunction. But furthermore, if one
  // write b Declare`...` and passes no function, the resolver itself
  // is the identity function. At that point, the types
  // TResolverParameter and TResolverReturn must be identical.

  return function identity<TResolverReturn>(
    x: (param: any) => TResolverReturn,
  ): (param: any) => TResolverReturn {
    return x;
  };
}