import { Suspense } from "react";
import Loader from "./loader";
import { Node } from "@/types/node";

export type NetworkData = {
	id: number;
	name: string;
	localIp: number;
	node: Node;
};

export default function NetworksList({ teamID }: { teamID: Promise<string>; }) {
	// todo: fallback skeleton list
	return (
		<Suspense>
			<Loader teamID={teamID} />
		</Suspense>
	);
}
