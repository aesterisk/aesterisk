import { ReactNode } from "react";
import Socket from "@/hooks/socket";
import { Toaster } from "@ui/sonner";

// todo: no mfa warning

export default async function Layout({
	children,
	params,
}: Readonly<{
	children: ReactNode;
	tabs: ReactNode;
	params: Promise<{ team: string; }>;
}>) {
	const team = params.then((p) => p.team);

	return (
		<Socket teamID={team}>
			{ children }
			<Toaster />
		</Socket>
	);
}
