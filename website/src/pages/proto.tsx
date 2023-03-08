/* eslint-disable sort-keys */

import React from 'react';
import { faLeaf, faScrewdriverWrench, faSolarSystem } from '@fortawesome/pro-duotone-svg-icons';
import { faBolt } from '@fortawesome/pro-regular-svg-icons';
import Features, { Feature } from '@site/src/components/Products/Features';
import Hero from '@site/src/components/Products/Proto/Hero';
import ToolsGrid from '@site/src/components/Products/Proto/ToolsGrid';
import CodeBlock from '@theme/CodeBlock';
import Layout from '@theme/Layout';
import Heading from '../ui/typography/Heading';
import Link from '../ui/typography/Link';
import Text from '../ui/typography/Text';

const toolchainFeatures: Feature[] = [
	{
		title: 'Lightspeed commands',
		icon: faBolt,
		description:
			'Download, install, and run tools with lightspeed, thanks to our Rust based foundation.',
	},
	{
		title: 'Universal toolchain',
		icon: faSolarSystem,
		description: 'Manage multiple languages and dependency managers through a single interface.',
	},
	{
		title: 'Ecosystem aware',
		icon: faLeaf,
		description: "Detects and infers from a language's ecosystem for maximum compatibility.",
	},
	{
		title: 'Granular configuration',
		icon: faScrewdriverWrench,
		description: 'Configure tools and their versions per directory, or per project.',
	},
];

export default function ProductProto() {
	return (
		<Layout
			title="proto - A language agnostic toolchain manager"
			description="Lightspeed toolchain manager for programming languages and their dependency managers."
		>
			<Hero />

			<main>
				<div className="bg-gradient-to-b from-slate-600 via-pink-900 to-white">
					<Features
						header="Toolchain"
						description="All in one"
						features={toolchainFeatures}
						tier={2}
					/>

					<div className="relative py-4 sm:py-5 lg:py-6">
						<div className="mx-auto max-w-md px-2 sm:max-w-3xl sm:px-3 lg:max-w-7xl lg:px-4">
							<div className="bg-white rounded-lg p-6 drop-shadow">
								<div className="grid grid-cols-2 gap-4 text-gray-900">
									<div>
										<Heading level={3} className="mb-2">
											Get started
										</Heading>

										<Text className="mb-1">Install proto for Linux, macOS, or WSL:</Text>

										<CodeBlock language="shell">
											{'curl -fsSL https://moonrepo.dev/install/proto.sh | bash'}
										</CodeBlock>

										<Text className="mb-1" variant="muted">
											Or Windows:
										</Text>

										<CodeBlock language="shell">
											{'irm https://moonrepo.dev/install/proto.ps1 | iex'}
										</CodeBlock>

										<Heading level={4} className="mt-4 mb-2">
											Install a tool
										</Heading>

										<CodeBlock language="shell">{'proto install node 18'}</CodeBlock>

										<Heading level={4} className="mt-4 mb-2">
											Run the tool
										</Heading>

										<CodeBlock language="shell">
											{'node ./main.mjs\n\n# Or with proto\nproto run node -- ./main.mjs'}
										</CodeBlock>
									</div>

									<div>
										<Heading level={3} className="mb-3">
											Supported tools
										</Heading>

										<ToolsGrid />

										<Heading level={3} className="mt-4 mb-2">
											Why another version manager?
										</Heading>

										<Text className="mb-2">
											To start, proto powers <Link href="/moon">moon's</Link> toolchain and
											integrated developer environment. We believed that the toolchain would be
											extremely beneficial for developers as a whole, and so we extracted proto out
											into a standalone Rust CLI, and{' '}
											<Link href="https://crates.io/users/milesj">Rust crates</Link> that moon
											inherits.
										</Text>

										<Text className="mb-2">
											Furthermore, we believe that requiring multiple ad-hoc version managers for
											all your languages, each with different workflows, CLI commands, and
											configuration files, is a poor developer experience.
										</Text>

										<Text>
											Our goal is to unify all of these into a single performant interface. A
											toolchain manager is the next step in the version manager evolution.
										</Text>
									</div>
								</div>
							</div>
						</div>
					</div>
				</div>
			</main>
		</Layout>
	);
}
