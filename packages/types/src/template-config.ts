// template.yml

export interface TemplateBooleanVariableConfig {
	default: boolean;
	prompt: string | null;
	required: boolean | null;
	type: 'boolean';
}

export interface TemplateEnumVariableConfig {
	default: string;
	multiple: boolean | null;
	prompt: string;
	type: 'enum';
	values: string[];
}

export interface TemplateNumberVariableConfig {
	default: number;
	prompt: string | null;
	required: boolean | null;
	type: 'number';
}

export interface TemplateStringVariableConfig {
	default: string;
	prompt: string | null;
	required: boolean | null;
	type: 'string';
}

export type TemplateVariable =
	| TemplateBooleanVariableConfig
	| TemplateEnumVariableConfig
	| TemplateNumberVariableConfig
	| TemplateStringVariableConfig;

export interface TemplateConfig {
	description: string;
	tite: string;
	variables: Record<string, TemplateVariable>;
}
