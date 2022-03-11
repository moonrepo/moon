import React from 'react';
import styles from './styles.module.css';

interface LabelProps {
	text: string;
}

export default function Label({ text }: LabelProps) {
	return <span className={styles.label}>{text}</span>;
}
