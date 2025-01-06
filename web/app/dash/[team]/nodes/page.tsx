import NodesList from "@/components/app/nodes-list";

export default function Nodes({ params }: { params: Promise<{ team: string; }>; }) {
	const teamID = params.then((p) => p.team);

	return (
		<main className="px-4 py-5">
			<NodesList teamID={teamID} />
		</main>
	);
}
