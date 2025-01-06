import { getTeam } from "@/caches/team";
import { getNodes } from "@/caches/nodes";
import Client from "./client";

export default async function Loader({ teamID }: { teamID: Promise<string>; }) {
	const team = await getTeam(await teamID);
	if(!team) throw new Error("NodesList requires a team");

	const nodes = await getNodes(team.team.id);

	return (
		<Client nodes={nodes} />
	);
}
