import { Suspense } from "react";
import Loader from "./loader";
import { Node } from "@/types/node";

export type ServerData = {
	id: number;
	name: string;
	tag: number;
	node: Node;
	status?: "healthy" | "starting" | "restarting" | "stopping" | "stopped" | "unhealthy";
	// todo: maybe add network stats?
	cpu?: {
		used: number;
		total: number;
	};
	memory?: {
		used: number;
		total: number;
	};
	storage?: {
		used: number;
		total: number;
	};
};

export default function ServersList({ teamID }: { teamID: Promise<string>; }) {
	// todo: fallback skeleton list
	return (
		<Suspense>
			<Loader teamID={teamID} />
		</Suspense>
	);
}
