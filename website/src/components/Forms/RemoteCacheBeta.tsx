/* eslint-disable promise/prefer-await-to-then */
import React, { ChangeEvent, ChangeEventHandler, FormEvent, useCallback, useState } from 'react';
import cx from 'clsx';

export interface FieldProps {
	label: string;
	name: string;
	value: string;
	onChange: ChangeEventHandler;
	type?: string;
}

export function Field({ label, name, value, onChange, type = 'text' }: FieldProps) {
	return (
		<>
			<label htmlFor={name} className="font-bold mb-0.5 block">
				{label}
			</label>

			<input
				type={type}
				name={name}
				id={name}
				required
				className="appearance-none outline-none min-w-0 bg-white border border-solid border-gray-400 dark:border-transparent rounded-md px-1 py-1 text-base text-gray-800 placeholder-gray-600 h-full font-sans w-5/6"
				onChange={onChange}
				value={value}
			/>
		</>
	);
}

export default function RemoteCacheBeta() {
	const [name, setName] = useState('');
	const [email, setEmail] = useState('');
	const [org, setOrg] = useState('');
	const [repo, setRepo] = useState('');
	const [region, setRegion] = useState('');
	const [sent, setSent] = useState(false);
	const disabled = !name || !email || !org || !repo || !region;

	const handleName = useCallback((event: ChangeEvent<HTMLInputElement>) => {
		setName(event.target.value);
	}, []);

	const handleEmail = useCallback((event: ChangeEvent<HTMLInputElement>) => {
		setEmail(event.target.value);
	}, []);

	const handleOrg = useCallback((event: ChangeEvent<HTMLInputElement>) => {
		setOrg(event.target.value);
	}, []);

	const handleRepo = useCallback((event: ChangeEvent<HTMLInputElement>) => {
		setRepo(event.target.value);
	}, []);

	const handleRegion = useCallback((event: ChangeEvent<HTMLSelectElement>) => {
		setRegion(event.target.value);
	}, []);

	const handleSubmit = useCallback(
		(event: FormEvent) => {
			event.preventDefault();

			void fetch('https://formspree.io/f/xeqdnjqr', {
				body: JSON.stringify({ email, name, org, region, repo }),
				headers: {
					Accept: 'application/json',
				},
				method: 'post',
			})
				// eslint-disable-next-line no-console
				.catch(console.error)
				.finally(() => {
					setSent(true);
				});
		},
		[email, name, org, region, repo],
	);

	if (sent) {
		return (
			<div className="mt-2 mb-4 font-bold">
				Thank you for signing up for the remote caching beta!
			</div>
		);
	}

	return (
		<form method="post" className="grid grid-cols-2 gap-3 mt-2 mb-4" onSubmit={handleSubmit}>
			<div className="col-span-1">
				<div>
					<Field label="Name" name="name" onChange={handleName} value={name} />
				</div>

				<div className="mt-2">
					<Field label="Email" name="email" onChange={handleEmail} value={email} type="email" />
				</div>

				<div className="mt-2">
					<label htmlFor="region" className="font-bold mb-0.5 block">
						Region
					</label>

					<select
						id="region"
						name="region"
						required
						className="outline-none min-w-0 bg-white border border-solid border-gray-400 dark:border-transparent rounded-md px-1 py-1 text-base text-gray-800 placeholder-gray-600 h-full font-sans w-5/6"
						onChange={handleRegion}
						value={region}
					>
						<option value="" disabled></option>
						<option value="north-america">North America</option>
						<option value="south-america">South America</option>
						<option value="europe">Europe</option>
						<option value="africa">Africa</option>
						<option value="asia">Asia</option>
						<option value="southeast-asia">Southeast Asia</option>
					</select>
				</div>
			</div>

			<div className="col-span-1">
				<div>
					<Field label="Organization" name="org" onChange={handleOrg} value={org} />
				</div>

				<div className="mt-2">
					<Field label="Repository URL" name="repo" onChange={handleRepo} value={repo} type="url" />
				</div>

				<div className="mt-2 pt-3">
					<button
						type="submit"
						className={cx(
							'w-1/4 border border-transparent rounded-md px-2 py-1 flex items-center justify-center text-base font-bold text-white bg-blurple-400 dark:bg-purple-600 mt-0.5',
							disabled
								? 'opacity-60'
								: 'hover:text-white hover:bg-blurple-500 dark:hover:bg-purple-500 cursor-pointer',
						)}
						disabled={disabled}
					>
						Sign up
					</button>
				</div>
			</div>
		</form>
	);
}
