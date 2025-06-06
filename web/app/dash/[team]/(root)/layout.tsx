import { ReactNode } from "react";
import type { Tab } from "@app/tab-bar/client";
import Navbar from "@/app/dash/navbar";

const tabs: Tab[] = [
	{
		name: "Overview",
		path: "/",
	},
	{
		name: "Servers",
		path: "/servers",
	},
	{
		name: "Projects",
		path: "/projects",
	},
	{
		name: "Networks",
		path: "/networks",
	},
	{
		name: "Nodes",
		path: "/nodes",
	},
	{
		name: "Domains",
		path: "/domains",
	},
	{
		name: "Templates",
		path: "/templates",
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
			<Navbar tabs={tabs} team={team} />
			{ children }
		</>
	);
}
