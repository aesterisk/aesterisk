import { getAccount } from "@/caches/account";
import { getTeam } from "@/caches/team";
import { Button } from "@ui/button";
import { Skeleton } from "@ui/skeleton";
import { ChevronsUpDown } from "lucide-react";
import { redirect } from "next/navigation";
import { Suspense } from "react";
import { TeamSwitcherInternal } from "./client";

function Loading() {
	return (
		<Button
			variant="outline"
			role="combobox"
			aria-label="Select a team"
			className="w-full justify-between px-[13px]"
		>
			<Skeleton className="h-5 w-5 mr-[11px] rounded-full" />
			<Skeleton className="h-2 w-28" />
			<ChevronsUpDown className="ml-auto h-4 w-4 shrink-0 opacity-50" />
		</Button>
	);
}

async function Loader({ team: teamPath, className }: {
	team: Promise<string>;
	className?: string;
}) {
	const switchTeam = async(team: string) => {
		"use server";
		redirect(`/dash/${team}`);
	};

	const account = await getAccount();
	if(!account) redirect("/auth/login");

	const team = await getTeam(await teamPath);

	return (
		<TeamSwitcherInternal
			selectedTeam={team}
			personalTeam={account.personalTeam}
			otherTeams={account.otherTeams}
			action={switchTeam}
			className={className}
		/>
	);
}

export function TeamSwitcher({ team, className }: {
	team: Promise<string>;
	className?: string;
}) {
	return (
		<Suspense fallback={<Loading />}>
			<Loader team={team} className={className} />
		</Suspense>
	);
}
