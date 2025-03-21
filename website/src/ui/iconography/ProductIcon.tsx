import Icon, { IconProps} from './Icon';

const icons = {
	discord: "codicon:comment-discussion",
	github: "codicon:github-inverted",
	moon: "material-symbols:moon-stars",
	'new-project': "material-symbols:add-box",
	'new-task': "material-symbols:add-circle",
	project: "material-symbols:inbox",
	'project-config': "material-symbols:inbox-customize",
	'project-config-global': "material-symbols:inbox-customize",
	'project-graph': "material-symbols:graph-2",
	'run-task': "material-symbols:play-circle",
	task: "material-symbols:circle",
	'task-config': "material-symbols:build-circle",
	token: "material-symbols:code",
	toolchain: "material-symbols:service-toolbox",
	'toolchain-config': "material-symbols:settings-alert",
	twitter: "codicon:twitter",
	workspace: "material-symbols:graph-5",
	'workspace-config': "material-symbols:settings",
};

export type ProductIconName = keyof typeof icons;

export interface ProductIconProps extends Omit<IconProps, 'icon'> {
	name: ProductIconName;
}

export default function ProductIcon({ name, ...props }: ProductIconProps) {
	return <Icon {...props} icon={icons[name]} />;
}
