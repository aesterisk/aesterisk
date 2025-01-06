import { getAccount } from "@/caches/account";
import { getTeam } from "@/caches/team";
import { redirect } from "next/navigation";
import Client from "./client";

export default async function Loader({ teamID }: { teamID: Promise<string>; }) {
	const switchTeam = async(team: string) => {
		"use server";
		redirect(`/dash/${team}`);
	};

	const account = await getAccount();
	if(!account) redirect("/auth/login");

	const team = await getTeam(await teamID);

	return (
		<Client
			selectedTeam={team}
			personalTeam={account.personalTeam}
			otherTeams={account.otherTeams}
			action={switchTeam}
		/>
	);
}
