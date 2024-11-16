import List from "@/app/panel/[teamID]/(routes)/nodes/components/list/list";
import { Suspense } from "react";

export default function NodesPage() {
	return (
		<main className="p-4">
			{ /* todo: add a fallback loading table */ }
			<Suspense>
				<List />
			</Suspense>
		</main>
	);
}
