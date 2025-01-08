import { Suspense } from "react";
import Loader from "./loader";

export type NodeData = {
	uuid: string;
	name: string;
	online?: boolean;
	lastActive: number;
	servers?: {
		online: number;
		failed: number;
		offline: number;
	};
	memory?: {
		used?: number;
		total?: number;
	};
	cpu?: number;
	storage?: {
		used?: number;
		total?: number;
	};
};

export default function NodesList({ teamID }: { teamID: Promise<string>; }) {
	// todo: fallback skeleton list
	return (
		<Suspense>
			<Loader teamID={teamID} />
		</Suspense>
	);
}
