import { ReactNode } from "react";
import type { Tab } from "@app/tab-bar/client";
import Navbar from "@/app/dash/navbar";

const tabs: Tab[] = [
	{
		name: "Overview",
		path: "/",
	},
	{
		name: "Console",
		path: "/console",
	},
	{
		name: "Logs",
		path: "/logs",
	},
	{
		name: "Files",
		path: "/files",
	},
	{
		name: "Environment",
		path: "/environment",
	},
	{
		name: "Networks",
		path: "/networks",
	},
	{
		name: "Activity",
		path: "/activity",
	},
	{
		name: "Settings",
		path: "/settings",
	},
];

export default function Layout({ children, params }: Readonly<{
	children: ReactNode;
	params: Promise<{ team: string; }>;
}>) {
	const team = params.then((p) => p.team);

	return (
		<>
			<Navbar tabs={tabs} level={4} team={team} />
			{ children }
		</>
	);
}
