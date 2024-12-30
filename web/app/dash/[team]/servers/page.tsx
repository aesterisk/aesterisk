import TeamTest from "@/components/app/team-test";

export default function Servers({ params }: { params: Promise<{ team: string; }>; }) {
	const team = params.then((p) => p.team);

	return (
		<main className="px-4 py-5 w-full">
			<span>{ "Servers" }</span>
			<TeamTest teamID={team} />
		</main>
	);
}
