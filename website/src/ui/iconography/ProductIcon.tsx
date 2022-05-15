import React from 'react';
import { faDiscord, faGithub, faTwitter } from '@fortawesome/free-brands-svg-icons';
import {
	faCircle,
	faCircleBolt,
	faCirclePlay,
	faCirclePlus,
	faDiagramProject,
	faGrid2,
	faSliders,
	faSquare,
	faSquarePlus,
	faSquareSliders,
	faSquareSlidersVertical,
	faToolbox,
	faTriangle,
} from '@fortawesome/pro-regular-svg-icons';
import { FontAwesomeIconProps } from '@fortawesome/react-fontawesome';
import Icon from './Icon';

const icons = {
	discord: faDiscord,
	github: faGithub,
	'new-project': faSquarePlus,
	'new-task': faCirclePlus,
	project: faSquare,
	'project-config': faSquareSlidersVertical,
	'project-config-global': faSquareSliders,
	'project-graph': faDiagramProject,
	'run-task': faCirclePlay,
	task: faCircle,
	'task-config': faCircleBolt,
	token: faTriangle,
	toolchain: faToolbox,
	twitter: faTwitter,
	workspace: faGrid2,
	'workspace-config': faSliders,
};

export type ProductIconName = keyof typeof icons;

export interface ProductIconProps extends Omit<FontAwesomeIconProps, 'icon'> {
	name: ProductIconName;
}

export default function ProductIcon({ name, ...props }: ProductIconProps) {
	return <Icon {...props} icon={icons[name]} />;
}
