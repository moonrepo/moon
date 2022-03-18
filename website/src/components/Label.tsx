import React from 'react';
import cx from 'clsx';
import styles from './styles.module.css';

interface LabelProps {
	header?: boolean;
	text: string;
}

export default function Label({ header, text }: LabelProps) {
	return <span className={cx(styles.label, header && styles.labelHeader)}>{text}</span>;
}
