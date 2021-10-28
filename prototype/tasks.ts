abstract class Task {
	args: string[] = [];

	binary: string;

	constructor(binary: string) {
		this.binary = binary;
	}
}

class BuildTask extends Task {}

export function createBuildTask(bin: string, args: string[]) {}
