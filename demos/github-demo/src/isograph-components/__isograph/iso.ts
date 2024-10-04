import type { IsographEntrypoint, ResolverFirstParameter, Variables } from '@isograph/react';
import { type Actor__UserLink__param } from './Actor/UserLink/param_type';
import { type IssueComment__formattedCommentCreationDate__param } from './IssueComment/formattedCommentCreationDate/param_type';
import { type PullRequest__CommentList__param } from './PullRequest/CommentList/param_type';
import { type PullRequest__PullRequestLink__param } from './PullRequest/PullRequestLink/param_type';
import { type PullRequest__createdAtFormatted__param } from './PullRequest/createdAtFormatted/param_type';
import { type PullRequestConnection__PullRequestTable__param } from './PullRequestConnection/PullRequestTable/param_type';
import { type Query__Header__param } from './Query/Header/param_type';
import { type Query__HomePageList__param } from './Query/HomePageList/param_type';
import { type Query__HomePage__param } from './Query/HomePage/param_type';
import { type Query__PullRequestDetail__param } from './Query/PullRequestDetail/param_type';
import { type Query__PullRequest__param } from './Query/PullRequest/param_type';
import { type Query__RepositoryDetail__param } from './Query/RepositoryDetail/param_type';
import { type Query__RepositoryPage__param } from './Query/RepositoryPage/param_type';
import { type Query__UserDetail__param } from './Query/UserDetail/param_type';
import { type Query__UserPage__param } from './Query/UserPage/param_type';
import { type Repository__RepositoryLink__param } from './Repository/RepositoryLink/param_type';
import { type Starrable__IsStarred__param } from './Starrable/IsStarred/param_type';
import { type User__Avatar__param } from './User/Avatar/param_type';
import { type User__RepositoryList__param } from './User/RepositoryList/param_type';
import entrypoint_Query__HomePage from '../__isograph/Query/HomePage/entrypoint';
import entrypoint_Query__PullRequest from '../__isograph/Query/PullRequest/entrypoint';
import entrypoint_Query__RepositoryPage from '../__isograph/Query/RepositoryPage/entrypoint';
import entrypoint_Query__UserPage from '../__isograph/Query/UserPage/entrypoint';

// This is the type given to regular client fields.
// This means that the type of the exported iso literal is exactly
// the type of the passed-in function, which takes one parameter
// of type TParam.
type IdentityWithParam<TParam extends object> = <TClientFieldReturn, TVariables = Variables>(
  clientField: (param: TParam) => TClientFieldReturn
) => (param: ResolverFirstParameter<TParam, TVariables>) => TClientFieldReturn;

// This is the type given it to client fields with @component.
// This means that the type of the exported iso literal is exactly
// the type of the passed-in function, which takes two parameters.
// The first has type TParam, and the second has type TComponentProps.
//
// TComponentProps becomes the types of the props you must pass
// whenever the @component field is rendered.
type IdentityWithParamComponent<TParam extends object> = <
  TClientFieldReturn,
  TComponentProps = Record<string, never>,
  TVariables = Variables
>(
  clientComponentField: (data: TParam, componentProps: TComponentProps) => TClientFieldReturn
) => (data: ResolverFirstParameter<TParam, TVariables>, componentProps: TComponentProps) => TClientFieldReturn;

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
  param: T & MatchesWhitespaceAndString<'field Actor.UserLink', T>
): IdentityWithParamComponent<Actor__UserLink__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field IssueComment.formattedCommentCreationDate', T>
): IdentityWithParam<IssueComment__formattedCommentCreationDate__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field PullRequest.CommentList', T>
): IdentityWithParamComponent<PullRequest__CommentList__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field PullRequest.PullRequestLink', T>
): IdentityWithParamComponent<PullRequest__PullRequestLink__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field PullRequest.createdAtFormatted', T>
): IdentityWithParam<PullRequest__createdAtFormatted__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field PullRequestConnection.PullRequestTable', T>
): IdentityWithParamComponent<PullRequestConnection__PullRequestTable__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.Header', T>
): IdentityWithParamComponent<Query__Header__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.HomePageList', T>
): IdentityWithParamComponent<Query__HomePageList__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.HomePage', T>
): IdentityWithParamComponent<Query__HomePage__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.PullRequestDetail', T>
): IdentityWithParamComponent<Query__PullRequestDetail__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.PullRequest', T>
): IdentityWithParamComponent<Query__PullRequest__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.RepositoryDetail', T>
): IdentityWithParamComponent<Query__RepositoryDetail__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.RepositoryPage', T>
): IdentityWithParamComponent<Query__RepositoryPage__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.UserDetail', T>
): IdentityWithParamComponent<Query__UserDetail__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.UserPage', T>
): IdentityWithParamComponent<Query__UserPage__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Repository.RepositoryLink', T>
): IdentityWithParamComponent<Repository__RepositoryLink__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Starrable.IsStarred', T>
): IdentityWithParamComponent<Starrable__IsStarred__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field User.Avatar', T>
): IdentityWithParamComponent<User__Avatar__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field User.RepositoryList', T>
): IdentityWithParamComponent<User__RepositoryList__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.HomePage', T>
): typeof entrypoint_Query__HomePage;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.PullRequest', T>
): typeof entrypoint_Query__PullRequest;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.RepositoryPage', T>
): typeof entrypoint_Query__RepositoryPage;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.UserPage', T>
): typeof entrypoint_Query__UserPage;

export function iso(_isographLiteralText: string):
  | IdentityWithParam<any>
  | IdentityWithParamComponent<any>
  | IsographEntrypoint<any, any>
{
  throw new Error('iso: Unexpected invocation at runtime. Either the Babel transform ' +
      'was not set up, or it failed to identify this call site. Make sure it ' +
      'is being used verbatim as `iso`.');
}