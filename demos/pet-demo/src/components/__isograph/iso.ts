import type {IsographEntrypoint} from '@isograph/react';
import entrypoint_Query__HomeRoute from '../__isograph/Query/HomeRoute/entrypoint.ts'
import entrypoint_Query__PetDetailRoute from '../__isograph/Query/PetDetailRoute/entrypoint.ts'
import { ResolverParameterType as field_Pet__PetBestFriendCard } from './Pet/PetBestFriendCard/reader.ts'
import { ResolverParameterType as field_Pet__PetCheckinsCard } from './Pet/PetCheckinsCard/reader.ts'
import { ResolverParameterType as field_Pet__PetPhraseCard } from './Pet/PetPhraseCard/reader.ts'
import { ResolverParameterType as field_Pet__PetStatsCard } from './Pet/PetStatsCard/reader.ts'
import { ResolverParameterType as field_Pet__PetSummaryCard } from './Pet/PetSummaryCard/reader.ts'
import { ResolverParameterType as field_Pet__PetTaglineCard } from './Pet/PetTaglineCard/reader.ts'
import { ResolverParameterType as field_Pet__PetUpdater } from './Pet/PetUpdater/reader.ts'
import { ResolverParameterType as field_Query__HomeRoute } from './Query/HomeRoute/reader.ts'
import { ResolverParameterType as field_Query__PetDetailRoute } from './Query/PetDetailRoute/reader.ts'

type IdentityWithParam<TParam> = <TResolverReturn>(
  x: (param: TParam) => TResolverReturn
) => (param: TParam) => TResolverReturn;

type WhitespaceCharacter = ' ' | '\t' | '\n';
type Whitespace<In> = In extends `${WhitespaceCharacter}${infer In}`
  ? Whitespace<In>
  : In;

type MatchesWhitespaceAndString<
  TString extends string,
  T
> = Whitespace<T> extends `${TString}${string}` ? T : never;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.HomeRoute', T>
): typeof entrypoint_Query__HomeRoute;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.PetDetailRoute', T>
): typeof entrypoint_Query__PetDetailRoute;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetBestFriendCard', T>
): IdentityWithParam<field_Pet__PetBestFriendCard>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetCheckinsCard', T>
): IdentityWithParam<field_Pet__PetCheckinsCard>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetPhraseCard', T>
): IdentityWithParam<field_Pet__PetPhraseCard>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetStatsCard', T>
): IdentityWithParam<field_Pet__PetStatsCard>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetSummaryCard', T>
): IdentityWithParam<field_Pet__PetSummaryCard>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetTaglineCard', T>
): IdentityWithParam<field_Pet__PetTaglineCard>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetUpdater', T>
): IdentityWithParam<field_Pet__PetUpdater>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.HomeRoute', T>
): IdentityWithParam<field_Query__HomeRoute>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.PetDetailRoute', T>
): IdentityWithParam<field_Query__PetDetailRoute>;

export function iso(_queryText: string): IdentityWithParam<any> | IsographEntrypoint<any, any, any>{
  return function identity<TResolverReturn>(
    clientFieldOrEntrypoint: (param: any) => TResolverReturn,
  ): (param: any) => TResolverReturn {
    return clientFieldOrEntrypoint;
  };
}