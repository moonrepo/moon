/* eslint-disable promise/prefer-await-to-then */

import React, { ChangeEvent, useCallback, useState } from 'react';
import { faXmark } from '@fortawesome/pro-regular-svg-icons';
import Button, { ButtonProps } from '../../../ui/Button';
import Icon from '../../../ui/iconography/Icon';
import Link from '../../../ui/typography/Link';
import Text from '../../../ui/typography/Text';

function NextButton(props: Partial<ButtonProps>) {
	return <Button {...props} label="Next" id="contact-next" className="w-1/4" />;
}

export default function ContactForm() {
	const [step, setStep] = useState(1);
	const [subject, setSubject] = useState('');
	const [email, setEmail] = useState('');
	const [message, setMessage] = useState('');
	const [sending, setSending] = useState(false);
	const [failed, setFailed] = useState(false);

	const handleReset = useCallback(() => {
		setStep(1);
		setSubject('');
		setEmail('');
		setMessage('');
		setSending(false);
		setFailed(false);
	}, []);

	const handleNext = useCallback(() => {
		setStep((prev) => prev + 1);
	}, []);

	const handleSubject = useCallback((event: ChangeEvent<HTMLSelectElement>) => {
		setSubject(event.target.value);
	}, []);

	const handleEmail = useCallback((event: ChangeEvent<HTMLInputElement>) => {
		setEmail(event.target.value);
	}, []);

	const handleMessage = useCallback((event: ChangeEvent<HTMLTextAreaElement>) => {
		setMessage(event.target.value);
	}, []);

	const handleSubmit = useCallback(() => {
		setSending(true);

		void fetch('https://formspree.io/f/xnqrnvgw', {
			body: JSON.stringify({ email, message, subject }),
			headers: {
				Accept: 'application/json',
			},
			method: 'post',
		})
			.then((res) => {
				setFailed(!res.ok);
			})
			.catch(() => {
				setFailed(true);
			})
			.finally(() => {
				setSending(false);
				handleNext();
			});
	}, [email, message, subject, handleNext]);

	const isEmailValid = !!email.match(/^.+@.+$/);
	const isMessageValid = message.length > 10;

	return (
		<>
			{subject ? (
				<Text>
					<Link className="float-right text-lg px-1" onClick={handleReset}>
						<Icon icon={faXmark} />
					</Link>
					Contacting about <b>{subject}</b>
				</Text>
			) : (
				<Text variant="muted">Want to learn more about moonrepo? Have questions?</Text>
			)}

			<div className="mt-2">
				{step === 1 && (
					<div className="flex justify-between gap-x-1">
						<div className="w-3/4">
							<label htmlFor="subject" className="sr-only">
								Subject
							</label>

							<select
								id="subject"
								name="subject"
								required
								className="outline-none min-w-0 w-full bg-white border border-transparent rounded-md px-1 py-1 text-base text-gray-800 placeholder-gray-600 h-full font-sans"
								onChange={handleSubject}
								value={subject}
							>
								<option value=""></option>
								<option value="Consultation">Consultation</option>
								<option value="Partnership">Partnership</option>
								<option value="Affiliation">Affiliation</option>
							</select>
						</div>

						<NextButton disabled={!subject} onClick={handleNext} />
					</div>
				)}

				{step === 2 && (
					<div className="flex justify-between gap-x-1">
						<div className="w-3/4">
							<label htmlFor="email" className="sr-only">
								Email address
							</label>

							<input
								type="email"
								name="email"
								id="email"
								autoComplete="email"
								required
								className="appearance-none outline-none min-w-0 w-full bg-white border border-transparent rounded-md px-1 py-1 text-base text-gray-800 placeholder-gray-600 h-full font-sans"
								placeholder="Email address"
								onChange={handleEmail}
								value={email}
							/>
						</div>

						<NextButton disabled={!isEmailValid} onClick={handleNext} />
					</div>
				)}

				{step === 3 && (
					<div>
						<textarea
							id="message"
							name="message"
							required
							className="appearance-none outline-none min-w-0 w-full bg-white border border-transparent rounded-md px-1 py-1 text-base text-gray-800 placeholder-gray-600 font-sans"
							placeholder="Message..."
							onChange={handleMessage}
						/>

						<div className="flex justify-end">
							<NextButton
								disabled={!isMessageValid || sending}
								label="Send"
								onClick={handleSubmit}
							/>
						</div>
					</div>
				)}

				{step === 4 && (
					<div>
						<Text>
							{failed
								? 'Failed to send message. Please try again.'
								: "Thanks for contacting us! We'll get back to you as soon as possible."}
						</Text>
					</div>
				)}
			</div>
		</>
	);
}
