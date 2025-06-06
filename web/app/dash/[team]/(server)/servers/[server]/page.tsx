import { Button } from "@ui/button";
import Link from "next/link";
import { Suspense } from "react";

async function DisplayPromise({ value }: { value: Promise<string>; }) {
	const resolvedValue = await value;

	return (
		<pre className="font-mono">{ resolvedValue }</pre>
	);
}

export default function Server({ params }: Readonly<{
	params: Promise<{
		team: string;
		server: string;
	}>;
}>) {
	const team = params.then((p) => p.team);
	const server = params.then((p) => p.server);

	return (
		<main className="px-4 py-5 w-full flex flex-col">
			<Link href="/dash/personal/servers">
				<Button variant="link">{ "back to server list" }</Button>
			</Link>
			<span>{ "Server " }</span>
			<Suspense fallback={<pre className="font-mono">{ "(loading...)" }</pre>}>
				<DisplayPromise value={server} />
			</Suspense>
		</main>
	);
}
