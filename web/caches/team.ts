import { UserTeam } from "@/types/team";
import { getAccount } from "@/caches/account";
import { redirect } from "next/navigation";
import { unstable_cacheLife as cacheLife, unstable_cacheTag as cacheTag } from "next/cache";

async function getCachedTeam(path: string, personalTeam: UserTeam, otherTeams: UserTeam[]) {
	"use cache";
	cacheLife("hours");
	cacheTag(`team-${path}`);

	if(path === "personal") {
		return personalTeam;
	}

	const team = otherTeams.find((t) => t.team.path === path);

	// todo: display an error message if team is unavailable, with a link back to the personal team
	if(!team) redirect("/dash/personal");

	return team;
}

export async function getTeam(path: string): Promise<UserTeam | null> {
	const account = await getAccount();
	if(!account) redirect("/auth/login");

	return getCachedTeam(path, account.personalTeam, account.otherTeams);
}
