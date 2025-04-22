import { getTeam } from "@/caches/team";
import { getNodes } from "@/caches/nodes";
import { getServers } from "@/caches/servers";
import Client from "./client";
import { ServerData } from ".";

export default async function Loader({ teamID }: { teamID: Promise<string>; }) {
	const team = await getTeam(await teamID);
	if(!team) throw new Error("ServersList requires a team");

	const nodes = await getNodes(team.team.id);
	const servers = await getServers(team.team.id);

	const serverData: ServerData[] = servers.map((server) => ({
		id: server.id,
		name: server.name,
		node: nodes.find((node) => node.id === server.node) ?? null,
		tag: server.tag,
	})).filter((server): server is ServerData => server.node !== null);

	return (
		<Client nodes={nodes} teamID={team.team.id} servers={serverData} />
	);
}
