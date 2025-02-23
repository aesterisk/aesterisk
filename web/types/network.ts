import { sql } from "@/lib/sql";

export type Network = {
	id: number;
	name: string;
	localIp: number;
	node: number;
};

export function fromDB(row: Record<string, unknown>): Network {
	return {
		id: row.network_id as number,
		name: row.network_name as string,
		localIp: row.network_local_ip as number,
		node: row.node_id as number,
	} satisfies Network;
}

export async function getNetworkById(id: number): Promise<Network | null> {
	const rows = await sql`
		SELECT
			network_id,
			network_name,
			network_local_ip,
			node_id
		FROM aesterisk.networks
		LEFT JOIN aesterisk.node_networks ON networks.network_id = node_networks.network_id
		WHERE network_id = ${id}
	`;

	if(rows.length !== 1) {
		return null;
	}

	return fromDB(rows[0]);
}

export async function getNodeNetworks(node: number): Promise<Network[]> {
	const rows = await sql`
		SELECT
			networks.network_id,
			network_name,
			network_local_ip,
			node_id
		FROM aesterisk.networks
		LEFT JOIN aesterisk.node_networks ON networks.network_id = node_networks.network_id
		WHERE node_id = ${node}
	`;

	return rows.map(fromDB);
}

export async function getTeamNetworks(team: number): Promise<Network[]> {
	const rows = await sql`
		SELECT
			networks.network_id,
			network_name,
			network_local_ip,
			node_networks.node_id
		FROM aesterisk.networks
		LEFT JOIN aesterisk.node_networks ON networks.network_id = node_networks.network_id
		LEFT JOIN aesterisk.team_nodes ON node_networks.node_id = team_nodes.node_id
		WHERE team_id = ${team}
	`;

	return rows.map(fromDB);
}

export async function addNetworkToNode(node: number, name: string, localIp: number): Promise<Network> {
	const rows = await sql`
		INSERT INTO aesterisk.networks (
			network_name,
			network_local_ip
		) VALUES (
			${name},
			${localIp}
		) RETURNING *;
	`;

	if(rows.length !== 1) {
		throw new Error("SQL query for inserting network failed!");
	}

	const network = fromDB({
		node,
		...rows[0],
	});

	await sql`
		INSERT INTO aesterisk.node_networks (
			node_id,
			network_id
		) VALUES (
			${node},
			${network.id}
		);
	`;

	return network;
}
