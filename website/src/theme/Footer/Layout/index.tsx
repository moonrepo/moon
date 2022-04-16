import React from 'react';
import clsx from 'clsx';
import Heading from '../../../ui/typography/Heading';
import Text from '../../../ui/typography/Text';

export default function FooterLayout({ style, links, logo, copyright }) {
	// return (
	// 	<footer className={clsx('footer')}>
	// 		<div className="container container-fluid">
	// 			{links}
	// 			{(logo || copyright) && (
	// 				<div className="footer__bottom text--center">
	// 					{logo && <div className="margin-bottom--sm">{logo}</div>}
	// 					{copyright}
	// 				</div>
	// 			)}
	// 		</div>
	// 	</footer>
	// );

	return (
		<footer className="bg-gray-100 dark:bg-slate-600" aria-labelledby="footer-heading">
			<h2 id="footer-heading" className="sr-only">
				Footer
			</h2>

			<div className="max-w-7xl mx-auto py-3 px-2 sm:px-3 md:py-4 md:px-4 lg:px-6">
				<div className="lg:grid lg:grid-cols-2 lg:gap-4">
					<div className="grid grid-cols-3 gap-4">{links}</div>

					<div className="mt-4 lg:mt-0">
						<Heading level={6} transform="uppercase">
							Subscribe to our newsletter
						</Heading>

						<Text className="mt-2" variant="muted">
							The latest news, articles, and resources, sent to your inbox weekly.
						</Text>

						<form className="mt-2 sm:flex sm:max-w-md">
							<label htmlFor="email-address" className="sr-only">
								Email address
							</label>
							<input
								type="email"
								name="email-address"
								id="email-address"
								autoComplete="email"
								required
								className="appearance-none min-w-0 w-full bg-white border border-transparent rounded-md py-2 px-4 text-base text-gray-900 placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-gray-800 focus:ring-white focus:border-white focus:placeholder-gray-400"
								placeholder="Enter your email"
							/>
							<div className="mt-3 rounded-md sm:mt-0 sm:ml-3 sm:flex-shrink-0">
								<button
									type="submit"
									className="w-full bg-indigo-500 border border-transparent rounded-md py-2 px-4 flex items-center justify-center text-base font-medium text-white hover:bg-indigo-600 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-gray-800 focus:ring-indigo-500"
								>
									Subscribe
								</button>
							</div>
						</form>
					</div>
				</div>

				<div className="mt-3 pt-3 md:mt-4 md:pt-4 border-0 border-t border-solid border-gray-200 dark:border-slate-400 flex items-center justify-center">
					{/* 	<div className="flex space-x-6 md:order-2">
						navigation.social.map((item) => (
							<a key={item.name} href={item.href} className="text-gray-400 hover:text-gray-300">
								<span className="sr-only">{item.name}</span>
								<item.icon className="h-6 w-6" aria-hidden="true" />
							</a>
            ))
					</div> */}

					<Text className="mt-2 md:mt-0 md:order-1" variant="muted" size="sm" as="div">
						{copyright}
					</Text>
				</div>
			</div>
		</footer>
	);
}
