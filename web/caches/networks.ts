import { getTeamNetworks } from "@/types/network";
import { unstable_cacheLife as cacheLife, unstable_cacheTag as cacheTag } from "next/cache";

export async function getNetworks(teamID: number) {
	"use cache";
	cacheLife("minutes");
	cacheTag(`networks-${teamID}`);

	return await getTeamNetworks(teamID);
}
