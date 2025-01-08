"use server";

import { addNodeToTeam, Node } from "@/types/node";
import { revalidateTag } from "next/cache";

export async function insertNode(team: number, name: string, key: string): Promise<Node> {
	const node = await addNodeToTeam(team, name, key);

	revalidateTag(`nodes-${team}`);

	return node;
}
