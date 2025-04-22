import { getTeamServers } from "@/types/server";
import { unstable_cacheLife as cacheLife, unstable_cacheTag as cacheTag } from "next/cache";

export async function getServers(teamID: number) {
	"use cache";
	cacheLife("minutes");
	cacheTag(`servers-${teamID}`);

	return await getTeamServers(teamID);
}
