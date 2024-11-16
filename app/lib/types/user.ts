import { sql } from "../sql";

export type User = {
	id: number;
	account: number;
	team: number;
	joinedAt: number;
	owner: boolean;
	roles: number[];
	publicKey: string;
	privateKey: string;
};

export async function getUserById(id: number): Promise<User | null> {
	const rows = await sql`
		SELECT
			user_id,
			user_account,
			user_team,
			extract(epoch from user_joined_at) as user_joined_at,
			user_owner,
			user_public_key,
			user_private_key
		FROM aesterisk.users
		WHERE user_id = ${id}
	`;

	if(rows.length !== 1) {
		return null;
	}

	return {
		id,
		account: rows[0].user_account as number,
		team: rows[0].user_team as number,
		joinedAt: Number(rows[0].user_joined_at),
		owner: rows[0].user_owner as boolean,
		// todo: roles
		roles: [],
		publicKey: rows[0].user_public_key as string,
		privateKey: rows[0].user_private_key as string,
	} satisfies User;
}
