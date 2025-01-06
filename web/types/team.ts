import { sql } from "@/lib/sql";
import { User } from "./user";
import { generateKeys } from "@/lib/signing";

export type Team = {
	id: number;
	path: string;
	name: string;
	plan: "free" | "plus" | "pro" | "enterprise";
	isPersonal: boolean;
	createdAt: number;
};

export type UserTeam = {
	user: number;
	team: Team;
	joinedAt: number;
	owner: boolean;
	publicKey: string;
	privateKey: string;
	// todo: roles
};

function idToPlan(plan: 0 | 1 | 2 | 3): "free" | "plus" | "pro" | "enterprise" {
	switch(plan) {
		case 0: return "free";
		case 1: return "plus";
		case 2: return "pro";
		case 3: return "enterprise";
		default: throw new Error(`Unknown plan: ${plan}`);
	}
}

function planToId(plan: "free" | "plus" | "pro" | "enterprise"): 0 | 1 | 2 | 3 {
	switch(plan) {
		case "free": return 0;
		case "plus": return 1;
		case "pro": return 2;
		case "enterprise": return 3;
		default: throw new Error(`Unknown plan: ${plan}`);
	}
}

export function fromDB(row: Record<string, unknown>): Team {
	// todo: use zod or other validation strategy to ensure the rows are valid
	//       this may affect performance so we'll need to measure it

	return {
		id: row.team_id as number,
		path: row.team_path as string,
		name: row.team_name as string,
		plan: idToPlan(row.team_plan as 0 | 1 | 2 | 3),
		isPersonal: row.team_is_personal as boolean,
		createdAt: Number(row.team_created_at),
	};
}

export async function getTeamById(id: number): Promise<Team | null> {
	const rows = await sql`SELECT team_id, team_path, team_name, team_plan, team_is_personal, extract(epoch from team_created_at) as team_created_at FROM aesterisk.teams WHERE team_id = ${id}`;
	if(rows.length === 0) return null;
	return fromDB(rows[0]);
}

export async function getTeamByPath(path: string): Promise<Team | null> {
	const rows = await sql`SELECT team_id, team_path, team_name, team_plan, team_is_personal, extract(epoch from team_created_at) as team_created_at FROM aesterisk.teams WHERE team_path = ${path}`;
	if(rows.length === 0) return null;
	return fromDB(rows[0]);
}

export async function addAccountToTeam(account: number, team: number, owner: boolean): Promise<User> {
	const { publicKey, privateKey } = await generateKeys();

	const rows = await sql`
		INSERT INTO aesterisk.users (
			user_account,
			user_team,
			user_joined_at,
			user_owner,
			user_public_key,
			user_private_key
		) VALUES (
			${account},
			${team},
			CURRENT_TIMESTAMP,
			${owner},
			${publicKey},
			${privateKey}
		) RETURNING
			user_id,
			extract(epoch from user_joined_at) as user_joined_at
	`;

	if(rows.length !== 1) {
		throw new Error("User not added to team"); // todo: add more context
	}

	return {
		id: rows[0].user_id as number,
		account,
		team,
		owner,
		roles: [],
		joinedAt: Number(rows[0].user_joined_at),
		publicKey,
		privateKey,
	} satisfies User;
}

export async function createTeam(path: string, name: string, plan: "free" | "plus" | "pro" | "enterprise", isPersonal: boolean, owner: number | null) {
	const rows = await sql`INSERT INTO aesterisk.teams (team_path, team_name, team_plan, team_created_at, team_is_personal) VALUES (${path}, ${name}, ${planToId(plan)}, CURRENT_TIMESTAMP, ${isPersonal}) RETURNING team_id, team_path, team_name, team_plan, team_is_personal, extract(epoch from team_created_at) as team_created_at`;

	if(rows.length !== 1) {
		throw new Error("Team not created"); // todo: add more context
	}

	const team = fromDB(rows[0]);

	if(owner !== null) await addAccountToTeam(owner, team.id, true);

	return team;
}

export async function createPersonalTeam(): Promise<Team> {
	return await createTeam("personal", "Personal", "free", true, null);
}
