/* eslint-disable */
import * as types from './graphql';
import { TypedDocumentNode as DocumentNode } from '@graphql-typed-document-node/core';

const documents = {
    "\n\tquery GetProjectInformation {\n\t\tworkspaceInfo {\n\t\t\tedges {\n\t\t\t\tid\n\t\t\t\tsource\n\t\t\t\ttarget\n\t\t\t}\n\t\t\tnodes {\n\t\t\t\tid\n\t\t\t\tlabel\n\t\t\t}\n\t\t}\n\t}\n": types.GetProjectInformationDocument,
};

export function graphql(source: "\n\tquery GetProjectInformation {\n\t\tworkspaceInfo {\n\t\t\tedges {\n\t\t\t\tid\n\t\t\t\tsource\n\t\t\t\ttarget\n\t\t\t}\n\t\t\tnodes {\n\t\t\t\tid\n\t\t\t\tlabel\n\t\t\t}\n\t\t}\n\t}\n"): (typeof documents)["\n\tquery GetProjectInformation {\n\t\tworkspaceInfo {\n\t\t\tedges {\n\t\t\t\tid\n\t\t\t\tsource\n\t\t\t\ttarget\n\t\t\t}\n\t\t\tnodes {\n\t\t\t\tid\n\t\t\t\tlabel\n\t\t\t}\n\t\t}\n\t}\n"];

export function graphql(source: string): unknown;
export function graphql(source: string) {
  return (documents as any)[source] ?? {};
}

export type DocumentType<TDocumentNode extends DocumentNode<any, any>> = TDocumentNode extends DocumentNode<  infer TType,  any>  ? TType  : never;