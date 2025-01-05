import { getTeamNodes } from "@/types/node";
import { unstable_cacheLife as cacheLife, unstable_cacheTag as cacheTag } from "next/cache";

export async function getNodes(teamID: number) {
	"use cache";
	cacheLife("minutes");
	cacheTag(`nodes-${teamID}`);

	return await getTeamNodes(teamID);
}
