import TeamTest from "@app/team-test";

export default function Home({ params }: Readonly<{ params: Promise<{ team: string; }>; }>) {
	const team = params.then((p) => p.team);

	return (
		<main className="px-4 py-5 w-full">
			<span>{ "Hello, world!" }</span>
			<TeamTest team={team} />
		</main>
	);
}
