import type {IsographEntrypoint} from '@isograph/react';
import { Checkin__CheckinDisplay__param } from './Checkin/CheckinDisplay/param_type';
import { Pet__FavoritePhraseLoader__param } from './Pet/FavoritePhraseLoader/param_type';
import { Pet__PetBestFriendCard__param } from './Pet/PetBestFriendCard/param_type';
import { Pet__PetCheckinsCard__param } from './Pet/PetCheckinsCard/param_type';
import { Pet__PetPhraseCard__param } from './Pet/PetPhraseCard/param_type';
import { Pet__PetStatsCard__param } from './Pet/PetStatsCard/param_type';
import { Pet__PetSummaryCard__param } from './Pet/PetSummaryCard/param_type';
import { Pet__PetTaglineCard__param } from './Pet/PetTaglineCard/param_type';
import { Pet__PetUpdater__param } from './Pet/PetUpdater/param_type';
import { Query__HomeRoute__param } from './Query/HomeRoute/param_type';
import { Query__PetDetailRoute__param } from './Query/PetDetailRoute/param_type';
import { Query__PetFavoritePhrase__param } from './Query/PetFavoritePhrase/param_type';
import entrypoint_Query__HomeRoute from '../__isograph/Query/HomeRoute/entrypoint';
import entrypoint_Query__PetDetailRoute from '../__isograph/Query/PetDetailRoute/entrypoint';
import entrypoint_Query__PetFavoritePhrase from '../__isograph/Query/PetFavoritePhrase/entrypoint';

type IdentityWithParam<TParam> = <TClientFieldReturn>(
  x: (param: TParam) => TClientFieldReturn
) => (param: TParam) => TClientFieldReturn;
type IdentityWithParamComponent<TParam> = <TClientFieldReturn, TSecondParam = Record<string, never>>(
  x: (data: TParam, secondParam: TSecondParam) => TClientFieldReturn
) => (data: TParam, secondParam: TSecondParam) => TClientFieldReturn;

type WhitespaceCharacter = ' ' | '\t' | '\n';
type Whitespace<In> = In extends `${WhitespaceCharacter}${infer In}`
  ? Whitespace<In>
  : In;

type MatchesWhitespaceAndString<
  TString extends string,
  T
> = Whitespace<T> extends `${TString}${string}` ? T : never;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Checkin.CheckinDisplay', T>
): IdentityWithParamComponent<Checkin__CheckinDisplay__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.FavoritePhraseLoader', T>
): IdentityWithParamComponent<Pet__FavoritePhraseLoader__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetBestFriendCard', T>
): IdentityWithParamComponent<Pet__PetBestFriendCard__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetCheckinsCard', T>
): IdentityWithParamComponent<Pet__PetCheckinsCard__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetPhraseCard', T>
): IdentityWithParamComponent<Pet__PetPhraseCard__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetStatsCard', T>
): IdentityWithParamComponent<Pet__PetStatsCard__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetSummaryCard', T>
): IdentityWithParamComponent<Pet__PetSummaryCard__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetTaglineCard', T>
): IdentityWithParamComponent<Pet__PetTaglineCard__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetUpdater', T>
): IdentityWithParamComponent<Pet__PetUpdater__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.HomeRoute', T>
): IdentityWithParamComponent<Query__HomeRoute__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.PetDetailRoute', T>
): IdentityWithParamComponent<Query__PetDetailRoute__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.PetFavoritePhrase', T>
): IdentityWithParamComponent<Query__PetFavoritePhrase__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.HomeRoute', T>
): typeof entrypoint_Query__HomeRoute;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.PetDetailRoute', T>
): typeof entrypoint_Query__PetDetailRoute;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.PetFavoritePhrase', T>
): typeof entrypoint_Query__PetFavoritePhrase;

export function iso(_isographLiteralText: string):
  | IdentityWithParam<any>
  | IdentityWithParamComponent<any>
  | IsographEntrypoint<any, any>
{
  return function identity<TClientFieldReturn>(
    clientFieldOrEntrypoint: (param: any) => TClientFieldReturn,
  ): (param: any) => TClientFieldReturn {
    return clientFieldOrEntrypoint;
  };
}