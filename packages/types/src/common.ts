export type Id = string;

export type Nullable<T> = { [K in keyof T]: T[K] | null };

export interface Duration {
	secs: number;
	nanos: number;
}

export interface ToolchainSpec {
	id: Id;
	overridden: boolean;
	req?: string | null;
}

export type ExtendsFrom = string[] | string;
