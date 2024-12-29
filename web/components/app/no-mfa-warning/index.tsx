import { Suspense } from "react";
import Client from "./client";
import { auth } from "@/lib/auth";

export default async function NoMFAWarning() {
	const session = auth();

	return (
		<Suspense>
			<Client mfaEnabled={(await session)?.user?.mfa ?? null} />
		</Suspense>
	);
}
