import { ReactNode, Suspense } from "react";
import { getTeam } from "@/caches/team";

async function Await({ value }: Readonly<{ value: Promise<ReactNode>; }>) {
	return await value;
}

export default async function TeamTest({ team }: Readonly<{ team: Promise<string>; }>) {
	const value = team.then((path) => getTeam(path)).then((data) => JSON.stringify(data, null, 4));

	return (
		<pre className="bg-accent text-accent-foreground p-4 rounded-md border w-full overflow-x-auto">
			<Suspense fallback="Loading...">
				<Await value={value} />
			</Suspense>
		</pre>
	);
}
