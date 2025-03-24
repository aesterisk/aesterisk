import ServersList from "@/components/app/servers-list";

export default function Servers({ params }: { params: Promise<{ team: string; }>; }) {
	const teamID = params.then((p) => p.team);

	return (
		<main className="px-4 py-5">
			<ServersList teamID={teamID} />
		</main>
	);
}
