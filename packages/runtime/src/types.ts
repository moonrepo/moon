import { PackageStructure, Path } from '@boost/common';

export interface TsConfigStructure {
	compilerOptions?: Record<string, unknown>;
	exclude?: string[];
	extends?: string;
	files?: string[];
	include?: string[];
	references?: { path: string }[];
}

// Keep in sync with crates/project/src/project.rs
export interface Project {
	id: string;
	package_json: PackageStructure | null;
	root: Path;
	source: string;
	tsconfig_json: TsConfigStructure | null;
}
