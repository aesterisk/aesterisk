import { Button } from "@ui/button";
import Link from "next/link";

export default function Servers({ params }: Readonly<{ params: Promise<{ team: string; }>; }>) {
	const team = params.then((p) => p.team);

	return (
		<main className="px-4 py-5 w-full flex flex-col">
			<span>{ "Servers" }</span>
			<Link href="/dash/personal/servers/0001">
				<Button variant="link">{ "go to server no 0001" }</Button>
			</Link>
		</main>
	);
}
