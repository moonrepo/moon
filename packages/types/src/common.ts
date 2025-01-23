import type { PlatformType } from './tasks-config';

export type Nullable<T> = { [K in keyof T]: T[K] | null };

export interface Duration {
	secs: number;
	nanos: number;
}

export interface Runtime {
	platform: PlatformType;
	requirement?: string;
	overridden?: boolean;
}

export type ExtendsFrom = string[] | string;

// 123456475
