import React from 'react';

export default function Screenshots() {
	return (
		<>
			<div className="overflow-hidden rounded-lg w-[100%] sm:w-[65%] lg:w-[65%] bg-[#000e19] p-1">
				<img src="/img/home/org.png" alt="moonbase - organization view" className="block" />
			</div>

			<div className="overflow-hidden rounded-lg w-[100%] sm:w-[65%] lg:w-[65%] bg-[#000e19] p-1 absolute bottom-0 right-0 z-10 hidden sm:block">
				<img src="/img/home/repo.png" alt="moonbase - repository view" className="block" />
			</div>
		</>
	);
}
