"use client";

import Link from "next/link";
import { ColumnDef } from "@tanstack/react-table";
import { Button } from "@/components/ui/button";
import { ServerData } from ".";

export const columns: ColumnDef<ServerData>[] = [
	{
		id: "name",
		header: () => <span className="w-min h-min select">{ "Server" }</span>,
		cell: ({ row }) => (
			// todo: link to /dash/[teamID]/servers/[serverID]
			<Link href="" className="select">{ row.original.name }</Link>
		),
		size: 300,
		minSize: 300,
	},
	{
		id: "node",
		header: () => <span className="w-min h-min select">{ "On Node" }</span>,
		cell: ({ row }) => (
			// todo: link to /dash/[teamID]/nodes/[nodeID]
			<Link href="" className="select">{ row.original.node.name }</Link>
		),
		size: 300,
		minSize: 300,
	},
	{
		id: "tag",
		header: () => <span className="w-min h-min select">{ "Tag" }</span>,
		cell: ({ row }) => (
			// todo: link to /dash/[teamID]/templates/[templateID]/tags/[tagID]
			<Link href="" className="select">{ row.original.tag }</Link>
		),
		size: 1000,
		minSize: 1000,
	},
	{
		id: "actions",
		cell: ({ row }) => (
			// todo: link to /dash/[teamID]/servers/[serverID]
			// todo: or a ... menu with options such as copy the docker id etc
			<Button size="sm" variant="outline">{ "Manage" }</Button>
		),
		minSize: 1,
		size: 1,
	},
];
