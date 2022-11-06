/* eslint-disable */
import { TypedDocumentNode as DocumentNode } from '@graphql-typed-document-node/core';
export type Maybe<T> = T | null;
export type InputMaybe<T> = Maybe<T>;
export type Exact<T extends { [key: string]: unknown }> = { [K in keyof T]: T[K] };
export type MakeOptional<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]?: Maybe<T[SubKey]> };
export type MakeMaybe<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]: Maybe<T[SubKey]> };
/** All built-in and custom scalars, mapped to their actual values */
export type Scalars = {
  ID: string;
  String: string;
  Boolean: boolean;
  Int: number;
  Float: number;
};

export type QueryRoot = {
  __typename?: 'QueryRoot';
  status: StatusDto;
  workspaceInfo: WorkspaceInfoDto;
};

export type StatusDto = {
  __typename?: 'StatusDto';
  isRunning: Scalars['Boolean'];
};

export type WorkspaceEdgeDto = {
  __typename?: 'WorkspaceEdgeDto';
  id: Scalars['String'];
  source: Scalars['Int'];
  target: Scalars['Int'];
};

export type WorkspaceInfoDto = {
  __typename?: 'WorkspaceInfoDto';
  edges: Array<WorkspaceEdgeDto>;
  nodes: Array<WorkspaceNodeDto>;
};

export type WorkspaceNodeDto = {
  __typename?: 'WorkspaceNodeDto';
  id: Scalars['Int'];
  label: Scalars['String'];
};

export type GetProjectInformationQueryVariables = Exact<{ [key: string]: never; }>;


export type GetProjectInformationQuery = { __typename?: 'QueryRoot', workspaceInfo: { __typename?: 'WorkspaceInfoDto', edges: Array<{ __typename?: 'WorkspaceEdgeDto', id: string, source: number, target: number }>, nodes: Array<{ __typename?: 'WorkspaceNodeDto', id: number, label: string }> } };


export const GetProjectInformationDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"GetProjectInformation"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"workspaceInfo"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"edges"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"source"}},{"kind":"Field","name":{"kind":"Name","value":"target"}}]}},{"kind":"Field","name":{"kind":"Name","value":"nodes"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"label"}}]}}]}}]}}]} as unknown as DocumentNode<GetProjectInformationQuery, GetProjectInformationQueryVariables>;