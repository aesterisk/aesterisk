import { UserTeam } from "@/lib/types/team";
import { getAccount } from "@/app/panel/caches/account";
import { redirect } from "next/navigation";
import { cache } from "react";

let currentPath = "personal";
export const setPath = (path: string) => {
	currentPath = path;
};

async function getTeamUncached(): Promise<UserTeam | null> {
	const account = await getAccount();
	if(!account) return null;

	if(currentPath === "personal") {
		return account.personalTeam;
	}

	const team = account.otherTeams.find((t) => t.team.path === currentPath);

	// todo: display an error message if team is unavailable, with a link back to the personal team
	if(!team) redirect("/panel/personal");

	return team;
}

export const getTeam = cache(getTeamUncached);
