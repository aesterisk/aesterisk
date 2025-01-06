import { Suspense } from "react";
import Loader from "./loader";
import Loading from "./loading";

export default function TeamSwitcher({ teamID }: { teamID: Promise<string>; }) {
	return (
		<Suspense fallback={<Loading />}>
			<Loader teamID={teamID} />
		</Suspense>
	);
}

