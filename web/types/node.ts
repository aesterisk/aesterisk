import { sql } from "@/lib/sql";

export type Node = {
	id: number;
	name: string;
	lastActive: number;
	publicKey: string;
	lastExternalIp: string;
	ipLocked: boolean;
	uuid: string;
	networkIpRange: string;
};

export function fromDB(row: Record<string, unknown>): Node {
	return {
		id: row.node_id as number,
		name: row.node_name as string,
		lastActive: Number(row.node_last_active_at),
		publicKey: row.node_public_key as string,
		lastExternalIp: row.node_last_external_ip as string,
		ipLocked: row.node_ip_locked as boolean,
		uuid: row.node_uuid as string,
		networkIpRange: row.node_network_ip_range as string,
	} satisfies Node;
}

export async function getNodeById(id: number): Promise<Node | null> {
	const rows = await sql`
		SELECT
			node_id,
			node_name,
			extract(epoch from node_last_active_at) as node_last_active_at,
			node_public_key,
			node_last_external_ip,
			node_ip_locked,
			node_uuid,
			node_network_ip_range
		FROM aesterisk.nodes
		WHERE node_id = ${id}
	`;

	if(rows.length !== 1) {
		return null;
	}

	return fromDB(rows[0]);
}

export async function getTeamNodes(team: number): Promise<Node[]> {
	const rows = await sql`
		SELECT
			nodes.node_id,
			node_name,
			extract(epoch from node_last_active_at) as node_last_active_at,
			node_public_key,
			node_last_external_ip,
			node_ip_locked,
			node_uuid,
			node_network_ip_range
		FROM aesterisk.nodes
		LEFT JOIN aesterisk.team_nodes ON nodes.node_id = team_nodes.node_id
		WHERE team_id = ${team}
	`;
	return rows.map(fromDB);
}

export async function addNodeToTeam(team: number, name: string, key: string): Promise<Node> {
	const rows = await sql`
		INSERT INTO aesterisk.nodes (
			node_name,
			node_last_active_at,
			node_last_external_ip,
			node_network_ip_range,
			node_public_key,
			node_uuid,
			node_ip_locked
		) VALUES (
			${name},
			CURRENT_TIMESTAMP,
			'0.0.0.0',
			2693,
			${key},
			gen_random_uuid(),
			false
		) RETURNING *;
	`;
	// todo: remove hard-coded network ip range, use a config value
	// todo: make last_active and last_external_ip fields nullable (cuz the node has never been active yet)

	if(rows.length !== 1) {
		throw new Error("SQL query for inserting node failed!");
	}

	const node = fromDB(rows[0]);

	await sql`
		INSERT INTO aesterisk.team_nodes (
			team_id,
			node_id
		) VALUES (
			${team},
			${node.id}
		);
	`;

	return node;
}
