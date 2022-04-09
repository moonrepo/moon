import React from 'react';
import {
	faCircle,
	faCircleBolt,
	faCircleDollar,
	faCirclePlus,
	faDiagramProject,
	faGrid2,
	faSliders,
	faSquare,
	faSquarePlus,
	faSquareSliders,
	faSquareSlidersVertical,
	faToolbox,
} from '@fortawesome/pro-regular-svg-icons';
import { FontAwesomeIconProps } from '@fortawesome/react-fontawesome';
import Icon from './Icon';

const icons = {
	'new-project': faSquarePlus,
	'new-task': faCirclePlus,
	project: faSquare,
	'project-config': faSquareSlidersVertical,
	'project-config-global': faSquareSliders,
	'project-graph': faDiagramProject,
	task: faCircle,
	'task-config': faCircleBolt,
	token: faCircleDollar,
	toolchain: faToolbox,
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
