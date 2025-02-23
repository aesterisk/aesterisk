"use server";

import { addNetworkToNode, Network } from "@/types/network";
import { revalidateTag } from "next/cache";

export async function insertNetwork(teamID: number, node: number, name: string, localIp: number): Promise<Network> {
	const network = await addNetworkToNode(node, name, localIp);

	revalidateTag(`networks-${teamID}`);

	return network;
}
