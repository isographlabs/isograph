import type { IsographEntrypoint } from '@isograph/react';
import { type AdItem__AdItemDisplay__param } from './AdItem/AdItemDisplay/param_type';
import { type BlogItem__BlogItemDisplay__param } from './BlogItem/BlogItemDisplay/param_type';
import { type BlogItem__BlogItemMoreDetail__param } from './BlogItem/BlogItemMoreDetail/param_type';
import { type Checkin__CheckinDisplay__param } from './Checkin/CheckinDisplay/param_type';
import { type Image__ImageDisplayWrapper__param } from './Image/ImageDisplayWrapper/param_type';
import { type Image__ImageDisplay__param } from './Image/ImageDisplay/param_type';
import { type Mutation__SetTagline__param } from './Mutation/SetTagline/param_type';
import { type NewsfeedItem__NewsfeedAdOrBlog__param } from './NewsfeedItem/NewsfeedAdOrBlog/param_type';
import { type Pet__Avatar__param } from './Pet/Avatar/param_type';
import { type Pet__FavoritePhraseLoader__param } from './Pet/FavoritePhraseLoader/param_type';
import { type Pet__FirstCheckinMakeSuperButton__param } from './Pet/FirstCheckinMakeSuperButton/param_type';
import { type Pet__PetBestFriendCard__param } from './Pet/PetBestFriendCard/param_type';
import { type Pet__PetCheckinsCardList__param } from './Pet/PetCheckinsCardList/param_type';
import { type Pet__PetCheckinsCard__param } from './Pet/PetCheckinsCard/param_type';
import { type Pet__PetDetailDeferredRouteInnerComponent__param } from './Pet/PetDetailDeferredRouteInnerComponent/param_type';
import { type Pet__PetPhraseCard__param } from './Pet/PetPhraseCard/param_type';
import { type Pet__PetStatsCard__param } from './Pet/PetStatsCard/param_type';
import { type Pet__PetSummaryCard__param } from './Pet/PetSummaryCard/param_type';
import { type Pet__PetTaglineCard__param } from './Pet/PetTaglineCard/param_type';
import { type Pet__PetUpdater__param } from './Pet/PetUpdater/param_type';
import { type Pet__Unreachable2__param } from './Pet/Unreachable2/param_type';
import { type Pet__UnreachableFromEntrypoint__param } from './Pet/UnreachableFromEntrypoint/param_type';
import { type Query__HomeRoute__param } from './Query/HomeRoute/param_type';
import { type Query__Newsfeed__param } from './Query/Newsfeed/param_type';
import { type Query__PetByName__param } from './Query/PetByName/param_type';
import { type Query__PetCheckinListRoute__param } from './Query/PetCheckinListRoute/param_type';
import { type Query__PetDetailDeferredRoute__param } from './Query/PetDetailDeferredRoute/param_type';
import { type Query__PetDetailRoute__param } from './Query/PetDetailRoute/param_type';
import { type Query__PetFavoritePhrase__param } from './Query/PetFavoritePhrase/param_type';
import { type Query__SmartestPetRoute__param } from './Query/SmartestPetRoute/param_type';
import { type Query__smartestPet__param } from './Query/smartestPet/param_type';
import { type Viewer__NewsfeedPaginationComponent__param } from './Viewer/NewsfeedPaginationComponent/param_type';
import entrypoint_Mutation__SetTagline from '../__isograph/Mutation/SetTagline/entrypoint';
import entrypoint_Query__HomeRoute from '../__isograph/Query/HomeRoute/entrypoint';
import entrypoint_Query__Newsfeed from '../__isograph/Query/Newsfeed/entrypoint';
import entrypoint_Query__PetByName from '../__isograph/Query/PetByName/entrypoint';
import entrypoint_Query__PetCheckinListRoute from '../__isograph/Query/PetCheckinListRoute/entrypoint';
import entrypoint_Query__PetDetailDeferredRoute from '../__isograph/Query/PetDetailDeferredRoute/entrypoint';
import entrypoint_Query__PetDetailRoute from '../__isograph/Query/PetDetailRoute/entrypoint';
import entrypoint_Query__PetFavoritePhrase from '../__isograph/Query/PetFavoritePhrase/entrypoint';
import entrypoint_Query__SmartestPetRoute from '../__isograph/Query/SmartestPetRoute/entrypoint';

// This is the type given to regular client fields.
// This means that the type of the exported iso literal is exactly
// the type of the passed-in function, which takes one parameter
// of type TParam.
type IdentityWithParam<TParam extends object> = <TClientFieldReturn>(
  clientField: (param: TParam) => TClientFieldReturn
) => (param: TParam) => TClientFieldReturn;

// This is the type given it to client fields with @component.
// This means that the type of the exported iso literal is exactly
// the type of the passed-in function, which takes two parameters.
// The first has type TParam, and the second has type TComponentProps.
//
// TComponentProps becomes the types of the props you must pass
// whenever the @component field is rendered.
type IdentityWithParamComponent<TParam extends object> = <
  TClientFieldReturn,
  TComponentProps = Record<PropertyKey, never>,
>(
  clientComponentField: (data: TParam, componentProps: TComponentProps) => TClientFieldReturn
) => (data: TParam, componentProps: TComponentProps) => TClientFieldReturn;

type WhitespaceCharacter = ' ' | '\t' | '\n';
type Whitespace<In> = In extends `${WhitespaceCharacter}${infer In}`
  ? Whitespace<In>
  : In;

// This is a recursive TypeScript type that matches strings that
// start with whitespace, followed by TString. So e.g. if we have
// ```
// export function iso<T>(
//   isographLiteralText: T & MatchesWhitespaceAndString<'field Query.foo', T>
// ): Bar;
// ```
// then, when you call
// ```
// const x = iso(`
//   field Query.foo ...
// `);
// ```
// then the type of `x` will be `Bar`, both in VSCode and when running
// tsc. This is how we achieve type safety — you can only use fields
// that you have explicitly selected.
type MatchesWhitespaceAndString<
  TString extends string,
  T
> = Whitespace<T> extends `${TString}${string}` ? T : never;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field AdItem.AdItemDisplay', T>
): IdentityWithParamComponent<AdItem__AdItemDisplay__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field BlogItem.BlogItemDisplay', T>
): IdentityWithParamComponent<BlogItem__BlogItemDisplay__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field BlogItem.BlogItemMoreDetail', T>
): IdentityWithParamComponent<BlogItem__BlogItemMoreDetail__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Checkin.CheckinDisplay', T>
): IdentityWithParamComponent<Checkin__CheckinDisplay__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Image.ImageDisplayWrapper', T>
): IdentityWithParamComponent<Image__ImageDisplayWrapper__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Image.ImageDisplay', T>
): IdentityWithParamComponent<Image__ImageDisplay__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Mutation.SetTagline', T>
): IdentityWithParamComponent<Mutation__SetTagline__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field NewsfeedItem.NewsfeedAdOrBlog', T>
): IdentityWithParamComponent<NewsfeedItem__NewsfeedAdOrBlog__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.Avatar', T>
): IdentityWithParamComponent<Pet__Avatar__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.FavoritePhraseLoader', T>
): IdentityWithParamComponent<Pet__FavoritePhraseLoader__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.FirstCheckinMakeSuperButton', T>
): IdentityWithParamComponent<Pet__FirstCheckinMakeSuperButton__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetBestFriendCard', T>
): IdentityWithParamComponent<Pet__PetBestFriendCard__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetCheckinsCardList', T>
): IdentityWithParam<Pet__PetCheckinsCardList__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetCheckinsCard', T>
): IdentityWithParamComponent<Pet__PetCheckinsCard__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetDetailDeferredRouteInnerComponent', T>
): IdentityWithParamComponent<Pet__PetDetailDeferredRouteInnerComponent__param>;

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
  param: T & MatchesWhitespaceAndString<'field Pet.Unreachable2', T>
): IdentityWithParam<Pet__Unreachable2__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.UnreachableFromEntrypoint', T>
): IdentityWithParam<Pet__UnreachableFromEntrypoint__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.HomeRoute', T>
): IdentityWithParamComponent<Query__HomeRoute__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.Newsfeed', T>
): IdentityWithParamComponent<Query__Newsfeed__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.PetByName', T>
): IdentityWithParamComponent<Query__PetByName__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.PetCheckinListRoute', T>
): IdentityWithParamComponent<Query__PetCheckinListRoute__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.PetDetailDeferredRoute', T>
): IdentityWithParamComponent<Query__PetDetailDeferredRoute__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.PetDetailRoute', T>
): IdentityWithParamComponent<Query__PetDetailRoute__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.PetFavoritePhrase', T>
): IdentityWithParamComponent<Query__PetFavoritePhrase__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.SmartestPetRoute', T>
): IdentityWithParamComponent<Query__SmartestPetRoute__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'pointer Query.smartestPet', T>
): IdentityWithParam<Query__smartestPet__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Viewer.NewsfeedPaginationComponent', T>
): IdentityWithParam<Viewer__NewsfeedPaginationComponent__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Mutation.SetTagline', T>
): typeof entrypoint_Mutation__SetTagline;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.HomeRoute', T>
): typeof entrypoint_Query__HomeRoute;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.Newsfeed', T>
): typeof entrypoint_Query__Newsfeed;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.PetByName', T>
): typeof entrypoint_Query__PetByName;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.PetCheckinListRoute', T>
): typeof entrypoint_Query__PetCheckinListRoute;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.PetDetailDeferredRoute', T>
): typeof entrypoint_Query__PetDetailDeferredRoute;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.PetDetailRoute', T>
): typeof entrypoint_Query__PetDetailRoute;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.PetFavoritePhrase', T>
): typeof entrypoint_Query__PetFavoritePhrase;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.SmartestPetRoute', T>
): typeof entrypoint_Query__SmartestPetRoute;

export function iso(_isographLiteralText: string):
  | IdentityWithParam<any>
  | IdentityWithParamComponent<any>
  | IsographEntrypoint<any, any, any>
{
  throw new Error('iso: Unexpected invocation at runtime. Either the Babel transform ' +
      'was not set up, or it failed to identify this call site. Make sure it ' +
      'is being used verbatim as `iso`. If you cannot use the babel transform, ' + 
      'set options.no_babel_transform to true in your Isograph config. ');
}