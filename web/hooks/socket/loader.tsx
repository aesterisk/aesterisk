import { getTeam } from "@/caches/team";
import React from "react";
import { SocketProvider } from "./client";

export default async function Loader({ children, teamID }: {
	children: React.ReactNode;
	teamID: Promise<string>;
}) {
	const team = await getTeam(await teamID);
	if(!team) throw new Error("Socket requires a team");

	return (
		<SocketProvider userID={team.user} publicKey={team.publicKey} privateKey={team.privateKey}>
			{ children }
		</SocketProvider>
	);
}
