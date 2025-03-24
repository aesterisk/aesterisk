import { Suspense } from "react";
import Loader from "./loader";
import { Node } from "@/types/node";

export type ServerData = {
	id: number;
	name: string;
	tag: number;
	node: Node;
};

export default function ServersList({ teamID }: { teamID: Promise<string>; }) {
	// todo: fallback skeleton list
	return (
		<Suspense>
			<Loader teamID={teamID} />
		</Suspense>
	);
}
