export type Platform = 'node' | 'system' | 'unknown';

export type Nullable<T> = { [K in keyof T]: T[K] | null };
