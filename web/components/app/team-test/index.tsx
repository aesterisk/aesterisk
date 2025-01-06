import { Suspense } from "react";
import { getTeam } from "@/caches/team";

export default async function TeamTest({ teamID }: { teamID: Promise<string>; }) {
	const team = teamID.then((id) => getTeam(id));

	return (
		<Suspense>
			<pre>{ JSON.stringify(await team, null, 4) }</pre>
		</Suspense>
	);
}
