export type Platform = 'node' | 'system' | 'unknown';

export type Nullable<T> = { [K in keyof T]: T[K] | null };

export interface Runtime {
	platform: Capitalize<Platform>;
	version?: string;
}
