import React from "react";
import Socket from "@/hooks/socket";
import { Sidebar } from "@/components/app/sidebar/components";
import AesteriskSidebar from "@/components/app/sidebar";
import AesteriskHeader from "@/components/app/header";
import NoMFAWarning from "@/components/app/no-mfa-warning";
import { Toaster } from "@/components/ui/sonner";

export default async function Layout({
	children,
	params,
}: Readonly<{
	children: React.ReactNode;
	params: Promise<{ team: string; }>;
}>) {
	const team = params.then((p) => p.team);

	return (
		<Socket teamID={team}>
			<div className="h-screen w-full md:grid-cols-[220px_1fr] lg:grid-cols-[280px_1fr] overflow-auto">
				<Sidebar>
					<AesteriskSidebar teamID={team} />
				</Sidebar>
				<div className="flex flex-col ml-[280px] mt-14">
					<div className="h-14 lg:h-[60px] w-[calc(100vw-280px)] bg-background fixed top-0 z-10" />
					<AesteriskHeader teamID={team} />
					{ children }
				</div>
			</div>
			<NoMFAWarning />
			<Toaster />
		</Socket>
	);
}
