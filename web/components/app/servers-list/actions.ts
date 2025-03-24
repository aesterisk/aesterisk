"use server";

import { addServerToNode, Server } from "@/types/server";
import { revalidateTag } from "next/cache";

export async function insertServer(teamID: number, node: number, name: string, tag: number): Promise<Server> {
	const server = await addServerToNode(node, name, tag);

	revalidateTag(`servers-${teamID}`);

	return server;
}
