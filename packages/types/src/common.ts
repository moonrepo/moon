export type Platform = 'deno' | 'node' | 'system' | 'unknown';

export type Nullable<T> = { [K in keyof T]: T[K] | null };

export interface Duration {
	secs: number;
	nanos: number;
}

export interface Runtime {
	platform: Capitalize<Platform>;
	version?: string;
}
