import { createPersonalTeam, addAccountToTeam, UserTeam, fromDB as teamFromDB } from "@/types/team";
import { sql } from "@/lib/sql";

export type Account = {
	id: number;
	email: string;
	firstName: string;
	lastName: string | null;
	avatar: string;
	createdAt: number;
	lastActive: number;
	personalTeam: UserTeam;
	otherTeams: UserTeam[];
};

export async function createAccount(
	ghId: string,
	email: string,
	firstName: string,
	lastName: string | null,
	image: string | null,
): Promise<Account> {
	const team = await createPersonalTeam();
	const rows = await sql`
		INSERT INTO aesterisk.accounts (
			account_gh_id,
			account_email,
			account_first_name,
			account_last_name,
			account_avatar,
			account_created_at,
			account_last_active_at,
			account_personal_team
		) VALUES (
			${ghId},
			${email},
			${firstName},
			${lastName},
			${image},
			CURRENT_TIMESTAMP,
			CURRENT_TIMESTAMP,
			${team.id}
		) RETURNING
			account_id,
			account_gh_id,
			account_email,
			account_first_name,
			account_last_name,
			account_avatar,
			extract(epoch from account_created_at) as account_created_at,
			extract(epoch from account_last_active_at) as account_last_active_at,
			account_personal_team
	`;

	if(rows.length !== 1) {
		throw new Error(`Account not created\nemail: ${email}\nfirstName: ${firstName}\nlastName: ${lastName}}`);
	}

	const user = await addAccountToTeam(rows[0].account_id as number, team.id, true);

	return {
		id: rows[0].account_id as number,
		email: rows[0].account_email as string,
		firstName: rows[0].account_first_name as string,
		lastName: rows[0].account_last_name as string,
		avatar: rows[0].account_avatar as string,
		createdAt: Number(rows[0].account_created_at),
		lastActive: Number(rows[0].account_last_active_at),
		personalTeam: {
			user: user.id as number,
			team,
			owner: true,
			joinedAt: Number(rows[0].account_created_at),
			publicKey: user.publicKey,
			privateKey: user.privateKey,
		},
		otherTeams: [],
	} satisfies Account;
}

export function userTeamFromDB(row: Record<string, unknown>): UserTeam {
	return {
		user: row.user_id as number,
		team: teamFromDB(row),
		owner: row.user_owner as boolean,
		joinedAt: Number(row.user_joined_at),
		publicKey: row.user_public_key as string,
		privateKey: row.user_private_key as string,
	} satisfies UserTeam;
}

export function fromDB(rows: Record<string, unknown>[]): Account {
	if(rows.length < 1) {
		throw new Error("Expected at least one row (account.ts)");
	}

	const personalTeam = rows.find((row) => row.team_is_personal === true);
	const otherTeams = rows.filter((row) => row.team_is_personal === false);

	return {
		id: rows[0].account_id as number,
		email: rows[0].account_email as string,
		firstName: rows[0].account_first_name as string,
		lastName: rows[0].account_last_name as string,
		avatar: rows[0].account_avatar as string,
		createdAt: Number(rows[0].account_created_at),
		lastActive: Number(rows[0].account_last_active_at),
		personalTeam: userTeamFromDB(personalTeam!),
		otherTeams: otherTeams.map(userTeamFromDB),
	} satisfies Account;
}

export async function getAccountByGhId(ghId: string): Promise<Account | null> {
	const rows = await sql`
		SELECT
			account_id,
			account_gh_id,
			account_email,
			account_first_name,
			account_last_name,
			account_avatar,
			extract(epoch from account_created_at) as account_created_at,
			extract(epoch from account_last_active_at) as account_last_active_at,
			user_id,
			extract(epoch from user_joined_at) as user_joined_at,
			user_owner,
			user_public_key,
			user_private_key,
			team_id,
			team_path,
			team_name,
			team_plan,
			team_is_personal,
			extract(epoch from team_created_at) as team_created_at
		FROM aesterisk.accounts
		LEFT JOIN aesterisk.users ON accounts.account_id = users.user_account
		LEFT JOIN aesterisk.teams ON users.user_team = teams.team_id
		WHERE accounts.account_gh_id = ${ghId}
	`;

	if(rows.length < 1) {
		return null;
	}

	return fromDB(rows);
}

export async function getAccountOrCreate(
	queryGhId: string,
	queryEmail: string,
	queryFirstName: string,
	queryLastName: string | null,
	queryImage: string | null,
): Promise<Account> {
	const acc = await getAccountByGhId(queryGhId);
	if(!acc) return await createAccount(queryGhId, queryEmail, queryFirstName, queryLastName, queryImage);
	return acc;
}
