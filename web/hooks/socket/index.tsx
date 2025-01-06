import React, { Suspense } from "react";
import Loader from "./loader";

export default function Socket({ children, teamID }: {
	children: React.ReactNode;
	teamID: Promise<string>;
}) {
	return (
		<Suspense fallback={children}>
			<Loader teamID={teamID}>
				{ children }
			</Loader>
		</Suspense>
	);
}
