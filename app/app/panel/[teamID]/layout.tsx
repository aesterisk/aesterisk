import React from "react";
import { getTeam, setPath } from "@/app/panel/caches/team";
import { getAccount } from "@/app/panel/caches/account";
import { SocketProvider } from "@/app/panel/[teamID]/hooks/socket";
import { auth } from "@/lib/auth";
import { redirect } from "next/navigation";
import { Sidebar } from "@/components/app/sidebar";
import AesteriskSidebar from "../components/sidebar";
import AesteriskHeader from "../components/header";
import NoMFAWarning from "../components/no-mfa-warning";

export default async function Layout({
	children,
	params: { teamID },
}: Readonly<{
	children: React.ReactNode;
	params: { teamID: string; };
}>) {
	setPath(teamID);

	const session = await auth();

	const team = await getTeam();
	const account = await getAccount();

	if(!session || !session.user || !team || !account) {
		redirect("/auth/login");
	}

	return (
		<SocketProvider userID={team.user} publicKey={account.publicKey} privateKey={account.privateKey}>
			<div className="h-screen w-full md:grid-cols-[220px_1fr] lg:grid-cols-[280px_1fr] overflow-auto">
				<Sidebar>
					<AesteriskSidebar />
				</Sidebar>
				<div className="flex flex-col ml-[280px] mt-14">
					<div className="h-14 lg:h-[60px] w-[calc(100vw-280px)] bg-background fixed top-0 z-10" />
					<AesteriskHeader />
					{ children }
				</div>
			</div>
			<NoMFAWarning mfaEnabled={session.user.twoFactor} />
		</SocketProvider>
	);
}
