import NetworksList from "@/components/app/networks-list";

export default function Networks({ params }: { params: Promise<{ team: string; }>; }) {
	const teamID = params.then((p) => p.team);

	return (
		<main className="px-4 py-5">
			<NetworksList teamID={teamID} />
		</main>
	);
}
