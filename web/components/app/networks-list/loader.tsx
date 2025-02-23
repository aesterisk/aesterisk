import { getTeam } from "@/caches/team";
import { getNodes } from "@/caches/nodes";
import { getNetworks } from "@/caches/networks";
import Client from "./client";
import { NetworkData } from ".";

export default async function Loader({ teamID }: { teamID: Promise<string>; }) {
	const team = await getTeam(await teamID);
	if(!team) throw new Error("NodesList requires a team");

	const nodes = await getNodes(team.team.id);
	const networks = await getNetworks(team.team.id);

	const networkData = networks.map((network) => ({
		id: network.id,
		name: network.name,
		node: nodes.find((node) => node.id === network.node)!,
		localIp: network.localIp,
	} satisfies NetworkData));

	return (
		<Client nodes={nodes} teamID={team.team.id} networks={networkData} />
	);
}
