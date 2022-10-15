// template.yml

export interface TemplateVariableConfig<T> {
	default: T;
	prompt: string | null;
	required: boolean | null;
}

export interface TemplateBooleanVariableConfig
	extends Omit<TemplateVariableConfig<boolean>, 'required'> {
	type: 'boolean';
}

export interface TemplateEnumValue {
	label: string;
	value: string;
}

export interface TemplateEnumVariableConfig
	extends Omit<TemplateVariableConfig<string>, 'prompt' | 'required'> {
	multiple: boolean | null;
	prompt: string;
	type: 'enum';
	values: (TemplateEnumValue | string)[];
}

export interface TemplateNumberVariableConfig extends TemplateVariableConfig<number> {
	type: 'number';
}

export interface TemplateStringVariableConfig extends TemplateVariableConfig<string> {
	type: 'string';
}

export type TemplateVariable =
	| TemplateBooleanVariableConfig
	| TemplateEnumVariableConfig
	| TemplateNumberVariableConfig
	| TemplateStringVariableConfig;

export interface TemplateConfig {
	description: string;
	title: string;
	variables: Record<string, TemplateVariable>;
}

export interface TemplateFrontmatterConfig {
	force: boolean | null;
	to: string | null;
	skip: boolean | null;
}
