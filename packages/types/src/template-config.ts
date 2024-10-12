// Automatically generated by schematic. DO NOT MODIFY!

/* eslint-disable */

/** Docs: https://moonrepo.dev/docs/config/template#frontmatter */
export interface TemplateFrontmatterConfig {
	/** @default 'https://moonrepo.dev/schemas/template-frontmatter.json' */
	$schema?: string;
	force: boolean;
	skip: boolean;
	to: string | null;
}

/** Docs: https://moonrepo.dev/docs/config/template#frontmatter */
export interface PartialTemplateFrontmatterConfig {
	/** @default 'https://moonrepo.dev/schemas/template-frontmatter.json' */
	$schema?: string | null;
	force?: boolean | null;
	skip?: boolean | null;
	to?: string | null;
}

/** Configuration for a template variable. */
export interface TemplateVariableBoolSetting {
	/** The default value of the variable if none was provided. */
	default: boolean;
	/** Marks the variable as internal, and won't be overwritten via CLI arguments. */
	internal: boolean;
	/** The order in which variables should be prompted for. */
	order: number | null;
	/** Prompt the user for a value when the generator is running. */
	prompt: string | null;
	/** Marks the variable as required, and will not accept an empty value. */
	required: boolean | null;
	type: 'boolean';
}

export type TemplateVariableEnumDefault = string | string[];

export interface TemplateVariableEnumValueConfig {
	/** A human-readable label for the value. */
	label: string;
	/** The literal enumerable value. */
	value: string;
}

export type TemplateVariableEnumValue = string | TemplateVariableEnumValueConfig;

export interface TemplateVariableEnumSetting {
	/** The default value of the variable if none was provided. */
	default: TemplateVariableEnumDefault;
	/** Marks the variable as internal, and won't be overwritten via CLI arguments. */
	internal: boolean;
	/** Allows multiple values to be selected. */
	multiple: boolean | null;
	/** The order in which variables should be prompted for. */
	order: number | null;
	/** Prompt the user for a value when the generator is running. */
	prompt: string | null;
	type: 'enum';
	/** List of acceptable values for this variable. */
	values: TemplateVariableEnumValue[];
}

/** Configuration for a template variable. */
export interface TemplateVariableNumberSetting {
	/** The default value of the variable if none was provided. */
	default: number;
	/** Marks the variable as internal, and won't be overwritten via CLI arguments. */
	internal: boolean;
	/** The order in which variables should be prompted for. */
	order: number | null;
	/** Prompt the user for a value when the generator is running. */
	prompt: string | null;
	/** Marks the variable as required, and will not accept an empty value. */
	required: boolean | null;
	type: 'number';
}

/** Configuration for a template variable. */
export interface TemplateVariableStringSetting {
	/** The default value of the variable if none was provided. */
	default: string;
	/** Marks the variable as internal, and won't be overwritten via CLI arguments. */
	internal: boolean;
	/** The order in which variables should be prompted for. */
	order: number | null;
	/** Prompt the user for a value when the generator is running. */
	prompt: string | null;
	/** Marks the variable as required, and will not accept an empty value. */
	required: boolean | null;
	type: 'string';
}

export type TemplateVariable = TemplateVariableBoolSetting | TemplateVariableEnumSetting | TemplateVariableNumberSetting | TemplateVariableStringSetting;

/**
 * Configures a template and its files to be scaffolded.
 * Docs: https://moonrepo.dev/docs/config/template
 */
export interface TemplateConfig {
	/** @default 'https://moonrepo.dev/schemas/template.json' */
	$schema?: string;
	/** A description on what the template scaffolds. */
	description: string;
	/**
	 * A pre-populated destination to scaffold to, relative from the
	 * workspace root.
	 */
	destination: string | null;
	/** Extends one or many other templates. */
	extends: string[];
	/** Overrides the ID of the template, instead of using the folder name. */
	id: string | null;
	/** A human-readable title for the template. */
	title: string;
	/**
	 * A mapping of variables that'll be interpolated within each template file.
	 * Variables can also be populated by passing command line arguments.
	 */
	variables: Record<string, TemplateVariable>;
}

/** Configuration for a template variable. */
export interface PartialTemplateVariableBoolSetting {
	/** The default value of the variable if none was provided. */
	default?: boolean | null;
	/** Marks the variable as internal, and won't be overwritten via CLI arguments. */
	internal?: boolean | null;
	/** The order in which variables should be prompted for. */
	order?: number | null;
	/** Prompt the user for a value when the generator is running. */
	prompt?: string | null;
	/** Marks the variable as required, and will not accept an empty value. */
	required?: boolean | null;
	type?: 'boolean' | null;
}

export type PartialTemplateVariableEnumDefault = string | string[];

export interface PartialTemplateVariableEnumValueConfig {
	/** A human-readable label for the value. */
	label?: string | null;
	/** The literal enumerable value. */
	value?: string | null;
}

export type PartialTemplateVariableEnumValue = string | PartialTemplateVariableEnumValueConfig;

export interface PartialTemplateVariableEnumSetting {
	/** The default value of the variable if none was provided. */
	default?: PartialTemplateVariableEnumDefault | null;
	/** Marks the variable as internal, and won't be overwritten via CLI arguments. */
	internal?: boolean | null;
	/** Allows multiple values to be selected. */
	multiple?: boolean | null;
	/** The order in which variables should be prompted for. */
	order?: number | null;
	/** Prompt the user for a value when the generator is running. */
	prompt?: string | null;
	type?: 'enum' | null;
	/** List of acceptable values for this variable. */
	values?: PartialTemplateVariableEnumValue[] | null;
}

/** Configuration for a template variable. */
export interface PartialTemplateVariableNumberSetting {
	/** The default value of the variable if none was provided. */
	default?: number | null;
	/** Marks the variable as internal, and won't be overwritten via CLI arguments. */
	internal?: boolean | null;
	/** The order in which variables should be prompted for. */
	order?: number | null;
	/** Prompt the user for a value when the generator is running. */
	prompt?: string | null;
	/** Marks the variable as required, and will not accept an empty value. */
	required?: boolean | null;
	type?: 'number' | null;
}

/** Configuration for a template variable. */
export interface PartialTemplateVariableStringSetting {
	/** The default value of the variable if none was provided. */
	default?: string | null;
	/** Marks the variable as internal, and won't be overwritten via CLI arguments. */
	internal?: boolean | null;
	/** The order in which variables should be prompted for. */
	order?: number | null;
	/** Prompt the user for a value when the generator is running. */
	prompt?: string | null;
	/** Marks the variable as required, and will not accept an empty value. */
	required?: boolean | null;
	type?: 'string' | null;
}

export type PartialTemplateVariable = PartialTemplateVariableBoolSetting | PartialTemplateVariableEnumSetting | PartialTemplateVariableNumberSetting | PartialTemplateVariableStringSetting;

/**
 * Configures a template and its files to be scaffolded.
 * Docs: https://moonrepo.dev/docs/config/template
 */
export interface PartialTemplateConfig {
	/** @default 'https://moonrepo.dev/schemas/template.json' */
	$schema?: string | null;
	/** A description on what the template scaffolds. */
	description?: string | null;
	/**
	 * A pre-populated destination to scaffold to, relative from the
	 * workspace root.
	 */
	destination?: string | null;
	/** Extends one or many other templates. */
	extends?: string[] | null;
	/** Overrides the ID of the template, instead of using the folder name. */
	id?: string | null;
	/** A human-readable title for the template. */
	title?: string | null;
	/**
	 * A mapping of variables that'll be interpolated within each template file.
	 * Variables can also be populated by passing command line arguments.
	 */
	variables?: Record<string, PartialTemplateVariable> | null;
}
