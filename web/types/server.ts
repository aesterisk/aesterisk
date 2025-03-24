import { sql } from "@/lib/sql";

export type Server = {
	id: number;
	name: string;
	tag: number;
	node: number;
};

export function fromDB(row: Record<string, unknown>): Server {
	return {
		id: row.server_id as number,
		name: row.server_name as string,
		tag: row.server_tag as number,
		node: row.node_id as number,
	} satisfies Server;
}

export async function getServerById(id: number): Promise<Server | null> {
	const rows = await sql`
		SELECT
			servers.server_id,
			server_name,
			server_tag,
			node_id
		FROM aesterisk.servers
		LEFT JOIN aesterisk.node_servers ON servers.server_id = node_servers.server_id
		WHERE server_id = ${id}
	`;

	if(rows.length !== 1) {
		return null;
	}

	return fromDB(rows[0]);
}

export async function getNodeServers(node: number): Promise<Server[]> {
	const rows = await sql`
		SELECT
			servers.server_id,
			server_name,
			server_tag,
			node_id
		FROM aesterisk.servers
		LEFT JOIN aesterisk.node_servers ON servers.server_id = node_servers.server_id
		WHERE node_id = ${node}
	`;

	return rows.map(fromDB);
}

export async function getTeamServers(team: number): Promise<Server[]> {
	const rows = await sql`
		SELECT
			servers.server_id,
			server_name,
			server_tag,
			node_servers.node_id
		FROM aesterisk.servers
		LEFT JOIN aesterisk.node_servers ON servers.server_id = node_servers.server_id
		LEFT JOIN aesterisk.team_nodes ON node_servers.node_id = team_nodes.node_id
		WHERE team_id = ${team}
	`;

	return rows.map(fromDB);
}

export async function addServerToNode(node: number, name: string, tag: number): Promise<Server> {
	const rows = await sql`
		INSERT INTO aesterisk.servers (
			server_name,
			server_tag
		) VALUES (
			${name},
			${tag}
		) RETURNING *;
	`;

	if(rows.length !== 1) {
		throw new Error("SQL query for inserting server failed!");
	}

	const server = fromDB({
		node,
		...rows[0],
	});

	await sql`
		INSERT INTO aesterisk.node_servers (
			node_id,
			server_id
		) VALUES (
			${node},
			${server.id}
		);
	`;

	return server;
}
